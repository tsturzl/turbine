mod log;
mod pagecache;

use tokio::{
    io,
    net::{TcpListener, TcpStream},
};
use byteorder::{ByteOrder, LittleEndian};
use rio::{Rio, Ordering};
use log::Log;
use std::sync::Arc;
use pagecache::common::{PAGE_SIZE, PageArr, Page};

#[tokio::main]
async fn main() -> io::Result<()> {
    let acceptor = TcpListener::bind("127.0.0.1:6142").await.expect("failed to start listener");

    let ring = rio::new().expect("rio started");

    server(ring, acceptor).await
}

async fn server(ring: Rio, acceptor: TcpListener) -> io::Result<()> {
    let log = Arc::new(Log::new("commit.log").await?);
    loop {
        let (stream, _) = acceptor.accept().await?;
        let ring = ring.clone();
        let log = log.clone();

        tokio::spawn(async move {
            let buf: [u8; 1] = [0; 1];
            let bytes_read = ring.read_at(&stream, &buf, 0).await?;
            if bytes_read == 0 {
                // if no bytes were read just skip
                return Ok(());
            }

            match buf[0] {
                // Noop/Ping
                0 => cmd_ping(&ring, &stream).await,
                // Read
                1 => cmd_read(log, &ring, &stream).await,
                // Write
                2 => cmd_write(log, &ring, &stream).await,
                _ => Ok::<_, io::Error>(()),
            }
        });
    }
}

async fn cmd_ping(ring: &Rio, stream: &TcpStream) -> io::Result<()> {
    ring.send_ordered(stream, &[0u8; 1], Ordering::Link).await?;
    Ok(())
}

// read a given chunk
async fn cmd_read(Log: Arc<Log>, ring: &Rio, stream: &TcpStream) -> io::Result<()> {
    let cmd_buf: [u8; 8] = [0; 8];
    let bytes_read = ring.read_at(stream, &cmd_buf, 0).await?;
    if bytes_read == 0 {
        // No command bytes provided, skip
        return Ok(());
    }
    let offset = LittleEndian::read_u64(&cmd_buf);
    
    let buf: PageArr = [0; PAGE_SIZE];
    
    // TODO: read logic

    Ok(())
}

// write a single chunk
async fn cmd_write(log: Arc<Log>, ring: &Rio, stream: &TcpStream) -> io::Result<()> {
    let cmd_buf: [u8; 4] = [0; 4];
    let bytes_read = ring.read_at(stream, &cmd_buf, 0).await?;
    if bytes_read == 0 {
        // No command bytes provided, skip
        return Ok(());
    }
    let bytes_incoming = LittleEndian::read_u64(&cmd_buf);
    // I could cast these to f64 and use ceil, but f64 could overflow. Instead do division...
    let mut pages_incoming = bytes_incoming / PAGE_SIZE as u64;
    // then if there is a remainder just round up
    if bytes_incoming % PAGE_SIZE as u64 != 0 {
        pages_incoming +=1 ;
    }

    {
        let mut pages: Vec<Page> = Vec::with_capacity(pages_incoming as usize);
        let lock = log.submition_lock.lock().await;
        // use a 64bit processor... or else
        for _ in 0..pages_incoming as usize {
            let buf: PageArr = [0; PAGE_SIZE];
            ring.read_at(stream, &buf, 0).await?;
            pages.push(log.new_page(ring, &buf).await?);
        }
        let flush_fut = log.flush(ring);
        drop(lock);

        flush_fut.await?;
    }

    let buf: PageArr = [0; PAGE_SIZE];

    let bytes_read = ring.read_at(stream, &buf, 0).await?;
    if bytes_read == 0 {
        // No bytes read, just skip
        return Ok(());
    }

    {
        // We lock to ensure order of the ring submition queue only
        let lock = log.submition_lock.lock().await;
        let page_fut = log.new_page(ring, &buf);
        let flush_fut = log.flush(ring);
        // drop the lock before we await the actual completion of IO work
        drop(lock);
        page_fut.await?;
        flush_fut.await?;
    }
    
    Ok(())
}
