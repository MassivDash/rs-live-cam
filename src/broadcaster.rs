use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};

use actix_web::web::{Bytes, Data};
use actix_web::Error;
use futures::Stream;
use image::codecs::jpeg::JpegEncoder;
use image::ExtendedColorType;
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[cfg(target_os = "windows")]
use image::{codecs::jpeg::JpegEncoder, ColorType};
#[cfg(not(target_os = "windows"))]
use opencv::{
    self,
    prelude::{MatTraitConst, VideoCaptureTrait, VideoCaptureTraitConst},
    videoio,
};

/// Hold clients channels
pub struct Broadcaster {
    clients: Vec<Sender<Bytes>>,
}

impl Broadcaster {
    fn new() -> Self {
        Broadcaster {
            clients: Vec::new(),
        }
    }

    pub fn create(width: u32, height: u32, fps: u64) -> Data<Mutex<Self>> {
        // Data ≃ Arc
        let me = Data::new(Mutex::new(Broadcaster::new()));

        Broadcaster::spawn_capture(me.clone(), width, height, fps);

        me
    }

    pub fn new_client(&mut self) -> Client {
        let (tx, rx) = channel(100);

        self.clients.push(tx);
        Client(rx)
    }

    fn make_message_block(frame: &[u8], width: u32, height: u32) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut encoder = JpegEncoder::new(&mut buffer);
        encoder
            .encode(frame, width, height, ExtendedColorType::Rgb8)
            .unwrap();

        let mut msg = format!(
            "--boundarydonotcross\r\nContent-Length:{}\r\nContent-Type:image/jpeg\r\n\r\n",
            buffer.len()
        )
        .into_bytes();
        msg.extend(buffer);
        msg
    }

    fn send_image(&mut self, msg: &[u8]) {
        let mut ok_clients = Vec::new();
        let msg = Bytes::from([msg].concat());
        for client in self.clients.iter() {
            let result = client.clone().try_send(msg.clone());

            if let Ok(()) = result {
                ok_clients.push(client.clone());
            }
        }
        self.clients = ok_clients;
    }

    #[cfg(target_os = "windows")]
    fn spawn_capture(me: Data<Mutex<Self>>, width: u32, height: u32, fps: u64) {
        let camera = escapi::init(0, width, height, fps).expect("Could not initialize the camera");
        let (width, height) = (camera.capture_width(), camera.capture_height());
        info!("actual (hiehgt, width) = ({}, {})", width, height);

        std::thread::spawn(move || loop {
            let pixels = camera.capture();

            let frame = match pixels {
                Ok(pixels) => {
                    // Lets' convert it to RGB.
                    let mut buffer = vec![0; width as usize * height as usize * 3];
                    for i in 0..pixels.len() / 4 {
                        buffer[i * 3] = pixels[i * 4 + 2];
                        buffer[i * 3 + 1] = pixels[i * 4 + 1];
                        buffer[i * 3 + 2] = pixels[i * 4];
                    }

                    buffer
                }
                _ => {
                    warn!("failed to capture");
                    vec![0; width as usize * height as usize * 3]
                }
            };

            let msg = Broadcaster::make_message_block(&frame, width, height);
            me.lock().unwrap().send_image(&msg);
        });
    }

    #[cfg(not(target_os = "windows"))]
    fn spawn_capture(
        me: Data<Mutex<Self>>,
        width: u32,
        height: u32,
        fps: u64,
    ) -> std::thread::JoinHandle<()> {
        let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY).unwrap(); // 0 is the default camera
       
        let opened = videoio::VideoCapture::is_opened(&cam).unwrap();

        cam.set(videoio::CAP_PROP_FRAME_WIDTH, width as f64)
            .unwrap();
        cam.set(videoio::CAP_PROP_FRAME_HEIGHT, height as f64)
            .unwrap();
        cam.set(videoio::CAP_PROP_FPS, fps as f64).unwrap();

        info!(
            "{}, {}, {}",
            cam.get(videoio::CAP_PROP_FRAME_WIDTH).unwrap(),
            cam.get(videoio::CAP_PROP_FRAME_HEIGHT).unwrap(),
            cam.get(videoio::CAP_PROP_FPS).unwrap()
        );

        std::thread::spawn(move || {
            if !opened {
                panic!("Unable to open default camera!");
            }
            let mut mat_frame = opencv::core::Mat::default();
            loop {
                
                cam.read(&mut mat_frame).unwrap();
                let mut frame = unsafe {
                    Vec::from(std::slice::from_raw_parts(
                        mat_frame.data(),
                        (width * height * 3) as usize,
                    ))
                };

                // Lets' convert from BGR to RGB.
                for i in 0..(width * height) {
                    frame.swap((i * 3) as usize, (i * 3 + 2) as usize);
                }

                let msg = Broadcaster::make_message_block(&frame, width, height);
                me.lock().unwrap().send_image(&msg);
            }
        })
    }
}

// wrap Receiver in own type, with correct error type
pub struct Client(Receiver<Bytes>);

impl Stream for Client {
    type Item = Result<Bytes, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_recv(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
