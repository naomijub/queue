use tokio::sync::{mpsc::Sender, oneshot::Sender as OsSender};

#[tokio::main]
async fn main() {
    let queue = Queue::start(300);

    queue.enqueue(35).await;
    queue.enqueue(45).await;
    queue.enqueue(55).await;

    println!("DEQUEUE 1 {:?}", queue.dequeue().await);
    println!("DEQUEUE 2 {:?}", queue.dequeue().await);
    println!("DEQUEUE 3 {:?}", queue.dequeue().await);
}

const MAX_CAPACITY: usize = 4096;

pub enum Action<T: Send + Clone> {
    Enqueue(T),
    Dequeue,
}

pub struct Queue<T: Send + Clone> {
    tx: Sender<(Action<T>, OsSender<Option<T>>)>,
}

impl<T: 'static + Send + Clone> Queue<T> {
    pub fn start(buffer: usize) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel(buffer);
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut elements: Vec<T> = Vec::new();
            loop {
                if let Some((action, o_tx)) = rx.recv().await {
                    let o_tx: OsSender<Option<T>> = o_tx;
                    match action {
                        Action::Enqueue(element) => {
                            if elements.len() >= MAX_CAPACITY {
                                if let Err(_) = o_tx.send(Some(element)) {
                                    println!("Receiver dropped at enqueue")
                                }
                            } else {
                                elements.push(element);
                                if let Err(_) = o_tx.send(None) {
                                    println!("Receiver dropped at enqueue")
                                }
                            }
                        }
                        Action::Dequeue => {
                            if elements.len() > 0 {
                                let removed_element = elements.remove(0);
                                if let Err(_) = o_tx.send(Some(removed_element)) {
                                    println!("Receiver dropped at dequeue")
                                }
                            } else {
                                if let Err(_) = o_tx.send(None) {
                                    println!("Receiver dropped at dequeue")
                                }
                            }
                        }
                    }
                }
            }
        });

        Queue { tx }
    }

    async fn enqueue(&self, value: T) -> Option<T> {
        let (os_tx, os_rx) = tokio::sync::oneshot::channel();
        let tx = self.tx.clone();
        let value_resp = value.clone();
        if let Err(_) = tx.send((Action::Enqueue(value), os_tx)).await {
            println!("Enqueue receiver dropped")
        }

        match os_rx.await {
            Ok(None) => None,
            _ => Some(value_resp.clone()),
        }
    }

    async fn dequeue(&self) -> Option<T> {
        let (os_tx, os_rx) = tokio::sync::oneshot::channel();
        let tx = self.tx.clone();
        if let Err(_) = tx.send((Action::Dequeue, os_tx)).await {
            println!("Dequeue receiver dropped")
        }

        match os_rx.await {
            Ok(Some(v)) => Some(v),
            _ => None,
        }
    }
}
