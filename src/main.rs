/*
 * TCP Chat Server
 *
 * @author Peter Ho
 * @date   Dec 18, 2020
 *
 * I pledge my honor that I have abided by the Stevens Honor System. -Ho
 */
use std::thread;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io::{Write, Read}; // For TcpStream traits
use std::sync::{mpsc, Mutex};
use std::ffi::CStr;

use crossbeam;

/*
 * Receive messages sent by the client for the current TcpStream
 */
fn recv(mut stream: TcpStream, addr: SocketAddr, tx: mpsc::Sender<String>) {
    println!("new client: {:?}", addr);
    loop {
        let mut buf = [0; 2048];
        match stream.read(&mut buf) {
            Ok(_bytes) => {
                // Send raw data formatted to String as SocketAddr;Text
                let res = unsafe { CStr::from_ptr(buf.as_ptr() as *mut i8) };
                if let Ok(buf) = res.to_str() {
                    let msg = format!("{:?};{}", stream.peer_addr(), buf);
                    if let Err(_) = tx.send(msg) {
                        println!("pipe broken");
                        break;
                    }
                } else {
                    println!("bad UTF-8. client disconnect?");
                    break;
                }
            }
            Err(err) => {
                panic!(err);
            }
        }
    }
}

/*
 * Send message out to client specified in stream
 */
fn send(mut stream: &TcpStream, msg: &String) {
    stream.write(msg[..].as_bytes()).unwrap();
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:5000").unwrap();

    /*
     * Prod/Cons pair is for clients sending
     * messages to the broadcast thread
     */
    let (tx, rx) = mpsc::channel::<String>();

    /*
     * Vector of client streams (sockets)
     * Used by both accept() thread and broadcast thread
     */
    let clients = Mutex::new(Vec::<TcpStream>::new());

    /*
     * Scope needed because Rust threads can live
     * without parent thread and need security
     * that stack vars don't get destroyed
     */
    crossbeam::scope(|s| {
        s.spawn(|_| {
            for msg in rx {
                // information is in SocketAddr;Text form
                let split = msg.splitn(2, ";").collect::<Vec<_>>();
                print!("Got: {}", split[1]);
                std::io::stdout().flush();

                // acquire mutex lock so accept() thread doesn't conflict
                let guard = clients.lock().unwrap();
                for client in guard.iter() {
                    if format!("{:?}", client.peer_addr()) != split[0] {
                        send(&client, &split[1].to_owned());
                    }
                }
            }
            thread::sleep(std::time::Duration::from_millis(100));
        });

        loop {
            match listener.accept() {
                Ok((stream, addr)) => {
                    let tx = tx.clone();
                    // need to clone because the new thread needs to take ownership of stack vars
                    clients.lock().unwrap().push(stream.try_clone().expect("couldn't clone"));
                    thread::spawn(move || {
                        recv(stream, addr, tx);
                    });
                }
                Err(err) => println!("error {:?}", err),
            }
        }
    }).unwrap();    // calls .join() on all s.spawn()'d threads
}
