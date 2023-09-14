use std::collections::HashMap;

use rand::Rng;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener,
    },
};

use tun::TunPacket;

mod mock_reply;

enum Message {
    Add((u16, OwnedWriteHalf)),
    Data((u16, Vec<u8>)),
    Remove(u16),
	#[allow(dead_code)]
	Mock((u16,Vec<u8>))
}

async fn read_body(len: u16, reader: &mut OwnedReadHalf) -> Result<Vec<u8>, std::io::Error> {
    let len = len as usize;
    let mut buf = Vec::new();
    buf.resize(len as usize, b'\0');
    let mut read_len = 0;
    loop {
        match reader.read(&mut buf[read_len..]).await {
            Ok(size) => {
                if size == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionAborted,
                        "",
                    ));
                }
                read_len += size;
                if read_len == len {
                    return Ok(buf);
                } else {
                    continue;
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

async fn find_another(map:& mut HashMap<u16,OwnedWriteHalf>,me:u16)->Option<& mut OwnedWriteHalf>{
	for i in map{
		if *i.0 != me{
			return Some(i.1);
		}
	}
	return None;	
}

#[tokio::main]
async fn main() {
    let listen = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let task = tokio::spawn(async move {
        let mut map = HashMap::new();
        loop {
            //println!("execute");
            match rx.recv().await {
                Some(Message::Add((num, writer))) => {
                    if map.len() >= 2 {
                        continue;
                    }
                    map.insert(num, writer);
                }
                Some(Message::Data((num, buff))) => {
                    println!("receive data from {num}:\n{buff:?}");
					match find_another(& mut map,num).await{
						Some(writer)=>{
							let mut w_buf = Vec::new();
							let len = (buff.len() as u16).to_be_bytes();
							w_buf.extend_from_slice(&len);
							w_buf.extend_from_slice(&buff);
							writer.write_all(&w_buf).await.unwrap();
						}
						None=>{}
					}
                }
                Some(Message::Remove(num)) => {
					println!("remove {num}");
                    map.remove(&num);
                }
				Some(Message::Mock((num,buff)))=>{
					println!("mock buff {buff:?}");
					let writer = map.get_mut(&num).unwrap();
					let time = rand::thread_rng().gen_range(100..500);
					tokio::time::sleep(std::time::Duration::from_millis(time)).await;
					mock_reply::parse_tun_packet(Some(Ok(TunPacket::new(buff))), writer).await;
					//writer.write_all(src)
				}
                None => {
					return;
				}
            };
        }
    });
    // let mut join_set = JoinSet::new();
    let mut index = 0;
    while let Ok((stream, _)) = listen.accept().await {
        let (mut reader, writer) = stream.into_split();
        tx.send(Message::Add((index, writer))).unwrap();
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut len_buf = [0u8; 2];
            let mut read_header_len = 0;
            loop {
                match reader.read(&mut len_buf[read_header_len..]).await {
                    Ok(size) => {
                        println!("read in comming {size}");
                        if size == 0 {
                            tx.send(Message::Remove(index)).unwrap();
                            return;
                        }
                        read_header_len += size;
                        if read_header_len == 2 {
                            read_header_len = 0;
                            let body_len = u16::from_be_bytes(len_buf);
                            println!("body len {body_len}");
                            match read_body(body_len, &mut reader).await {
                                Ok(buf) => {
                                    //println!("ready body:\n {buf:?}");
                                    tx.send(Message::Data((index, buf))).unwrap();
									//tx.send(Message::Mock((index, buf))).unwrap();
                                }
                                Err(_) => {
                                    tx.send(Message::Remove(index)).unwrap();
                                    return;
                                }
                            }
                        } else {
                            continue;
                        }
                    }
                    Err(_) => {
                        tx.send(Message::Remove(index)).unwrap();
                        return;
                    }
                }
            }
        });
        index += 1;
    }
	task.await.unwrap();
}
