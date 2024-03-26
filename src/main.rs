use opencv::{
    core::{Mat, Vector},
    imgcodecs,
    prelude::*,
    videoio,
};

use std::io::Write;
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("192.168.0.129:8000").unwrap();
    println!("Server listening on port 8080");

    let mut cam =
        videoio::VideoCapture::new(0, videoio::CAP_ANY).expect("Failed to get video capture");
    let mut frame = Mat::default();
    let mut buf = Vector::new();

    loop {
        let (mut stream, _) = listener.accept().expect("Failed to accept connection");

        cam.read(&mut frame).expect("Failed to capture frame");
        buf.clear();
        let _ = imgcodecs::imencode(".jpg", &frame, &mut buf, &Vector::new());

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=frame\r\n\r\n"
        );

        if stream.write_all(response.as_bytes()).is_err() {
            continue;
        }

        loop {
            cam.read(&mut frame).expect("Failed to capture frame");
            buf.clear();
            let _ = imgcodecs::imencode(".jpg", &frame, &mut buf, &Vector::new());

            let image_data = format!(
                "--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                buf.len()
            );

            if stream.write_all(image_data.as_bytes()).is_err() {
                break;
            }
            if stream.write_all(buf.as_slice()).is_err() {
                break;
            }
            if stream.write_all(b"\r\n").is_err() {
                break;
            }
            if stream.flush().is_err() {
                break;
            }
        }
    }
}
