use std::fs;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpListener;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;
use webserver_rust::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8000").unwrap(); // TCP 리스너 생성 및 주소 바인딩
    let pool = ThreadPool::new(4);

    for stream in listener.incoming() {
        // 새로운 TCP 연결이 들어올때마다 이터레이터 반환 Result<TcpStream,>
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down.");
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream); //TcpStream을 BufReader로 감싸서 읽기작업 수행
    let request_line = buf_reader.lines().next().unwrap().unwrap(); //BufReader에서 줄 단위로 데이터를 읽는 iter를 획득, 첫번째 줄을 읽어옴

    let (status_line, filename) = match request_line.as_str() {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let contents = fs::read_to_string(filename).unwrap();

    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    );

    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
