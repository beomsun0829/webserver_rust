use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>, // 작업을 실행하는 개별 워커(스레드)를 담는 벡터
    sender: Option<mpsc::Sender<Job>>, //메인 스레드에서 워커 스레드로 작업을 보내는 채널 송신자
}

//힙에 할당되어 있고, 한 번만 실행될 수 있으며, 스레드 간 이동이 안전하고, 프로그램 전 기간 유효한 어떤 클로저나 함수
type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Create a new ThreadPool.
    /// The size is the number of threads in the pool.
    /// # Panics
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel(); // 스레드 간 메세징을 위한 채널
        let receiver = Arc::new(Mutex::new(receiver)); // Arc(공유 소유권), Mutex(가변 접근 동기화), 하나의 데이터를 여러 스레드가 공유하면서 가변적으로 처리

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver))); //Arc::clone을 이용해 참조 카운트를 증가
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    // 실행할 작업 (클로저 f)을 ThreadPool의 워커 스레드 중 하나에게 보냄
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap(); //ThreadPool의 sender 필드에서 Sender 참조자를 꺼내 (as ref) Job을 채널로 보냄
    }
}

// ThreadPool 인스턴스가 스코프를 벗어나 소멸될 때
impl Drop for ThreadPool {
    fn drop(&mut self) {
        //ThreadPool의 sender 필드에서 Sender를 꺼내 소유권을 가져와 drop시킴
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            //JoinHandle을 꺼내 소유권을 가져옴, worker.thread는 None이 됨
            //JoinHandle이 존재할 경우 (Some) thread.join을 호출하여 워커 스레드가 완료될 때 까지 대기
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            // 클로저가 receiver의 소유권을 가지게 됨
            // 공유 receiver에 접근하기 위해 Mutex 잠금을 획득
            // 채널로부터 작업을 받음 (recv()), 채널이 작업이 없으면 스레드는 여기서 블로킹 됨, 채널의 모든 sender가 끊기면 Err를 반환
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");
                    job(); //pool.execute를 호출할 때의 파라미터인 클로저 리터럴 실행
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}
