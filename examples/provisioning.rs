mod common;

use mqttrust::{Mqtt, QoS, SubscribeTopic};
use mqttrust_core::{bbqueue::BBBuffer, EventLoop, MqttOptions, Notification, PublishNotification};

use common::clock::SysClock;
use common::file_handler::FileHandler;
use common::network::{Network, TcpSocket};
use native_tls::{TlsConnector, TlsStream};
use rustot::provisioning::{topics::Topic, Credentials, FleetProvisioner, Response};
use std::ops::DerefMut;
use std::{net::TcpStream, thread};

use common::credentials;

static mut Q: BBBuffer<{ 1024 * 6 }> = BBBuffer::new();

const THING_NAME: &str = "rustot-test";

pub struct OwnedCredentials {
    certificate_id: String,
    certificate_pem: String,
    private_key: Option<String>,
}

impl<'a> From<Credentials<'a>> for OwnedCredentials {
    fn from(c: Credentials<'a>) -> Self {
        Self {
            certificate_id: c.certificate_id.to_string(),
            certificate_pem: c.certificate_pem.to_string(),
            private_key: c.private_key.map(ToString::to_string),
        }
    }
}

fn provision_credentials<'a, const L: usize>(
    hostname: &'a str,
    mqtt_eventloop: &mut EventLoop<'a, 'a, TcpSocket<TlsStream<TcpStream>>, SysClock, L>,
    mqtt_client: &mqttrust_core::Client<L>,
) -> Result<OwnedCredentials, ()> {
    let connector = TlsConnector::builder()
        .identity(credentials::claim_identity())
        .add_root_certificate(credentials::root_ca())
        .build()
        .unwrap();

    let mut network = Network::new_tls(connector, String::from(hostname));

    mqtt_eventloop.options = MqttOptions::new(THING_NAME, hostname.into(), 8883);

    nb::block!(mqtt_eventloop.connect(&mut network))
        .expect("To connect to MQTT with claim credentials");

    log::info!("Successfully connected to broker with claim credentials");

    #[cfg(feature = "cbor")]
    let mut provisioner = FleetProvisioner::new(mqtt_client, "provision_template");
    #[cfg(not(feature = "cbor"))]
    let mut provisioner = FleetProvisioner::new_json(mqtt_client, "provision_template");
    provisioner
        .initialize()
        .expect("Failed to initialize FleetProvisioner");

    let mut provisioned_credentials: Option<OwnedCredentials> = None;

    let result = loop {
        match mqtt_eventloop.yield_event(&mut network) {
            Ok(Notification::Publish(mut publish)) if Topic::check(publish.topic_name.as_str()) => {
                let PublishNotification {
                    topic_name,
                    payload,
                    ..
                } = publish.deref_mut();

                match provisioner.handle_message::<4>(topic_name.as_str(), payload) {
                    Ok(Response::Credentials(credentials)) => {
                        log::info!("Got credentials! {:?}", credentials);
                        provisioned_credentials = Some(credentials.into());

                        let mut parameters = heapless::IndexMap::new();
                        parameters.insert("deviceId", THING_NAME).unwrap();

                        provisioner
                            .register_thing::<2>(Some(parameters))
                            .expect("To successfully publish to RegisterThing");
                    }
                    Ok(Response::DeviceConfiguration(config)) => {
                        // Store Device configuration parameters, if any.

                        log::info!("Got device config! {:?}", config);

                        break Ok(());
                    }
                    Ok(Response::None) => {}
                    Err(e) => {
                        log::error!("Got provision error! {:?}", e);

                        break Err(());
                    }
                }
            }
            Ok(Notification::Suback(_)) => {
                log::info!("Starting provisioning");
                provisioner.begin().expect("To begin provisioning");
            }
            Ok(n) => {
                log::trace!("{:?}", n);
            }
            _ => {}
        }
    };

    // Disconnect from AWS IoT Core
    mqtt_eventloop.disconnect(&mut network);

    result.and_then(|_| provisioned_credentials.ok_or(()))
}

fn main() {
    env_logger::init();

    let (p, c) = unsafe { Q.try_split_framed().unwrap() };

    log::info!("Starting provisioning example...");

    let mut mqtt_eventloop = EventLoop::new(
        c,
        SysClock::new(),
        MqttOptions::new(THING_NAME, "".into(), 8883),
    );

    let mqtt_client = mqttrust_core::Client::new(p, THING_NAME);

    // Connect to AWS IoT Core with provisioning claim credentials
    let hostname = credentials::HOSTNAME.unwrap();

    let credentials = provision_credentials(hostname, &mut mqtt_eventloop, &mqtt_client).unwrap();

    // TODO: PKCS#8 -> PKCS#12, or
    // https://github.com/sfackler/rust-native-tls/pull/209 whichever comes
    // first.
    let provisioned_identity = credentials::identity();

    // Connect to AWS IoT Core with provisioned certificate
    let connector = TlsConnector::builder()
        .identity(provisioned_identity)
        .add_root_certificate(credentials::root_ca())
        .build()
        .unwrap();

    let mut network = Network::new_tls(connector, String::from(hostname));
    mqtt_eventloop.options = MqttOptions::new(THING_NAME, hostname.into(), 8883);

    nb::block!(mqtt_eventloop.connect(&mut network))
        .expect("To connect to MQTT with provisioned credentials");

    log::info!("Successfully connected to broker with provisioned credentials");

    loop {
        thread::sleep(std::time::Duration::from_millis(5000));
    }
}
