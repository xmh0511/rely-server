use std::{collections::HashMap, sync::Arc, io::Write};
fn main(){
    let mut socket = std::net::TcpStream::connect("192.168.1.11:3000").unwrap();
    let buff = b"123456789abcdefg";
    socket.write_all(buff).unwrap();
}