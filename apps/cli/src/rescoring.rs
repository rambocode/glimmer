//! 异步重打分在 CLI 里的等法：壳是停顿后请求、定时器轮询，这里没有停顿，查询完直接请求并阻塞等结果，再查一次。

use std::time::{Duration, Instant};

use glimmer_core::{Engine, Query};

/// 最多等多久。
const WAIT: Duration = Duration::from_secs(5);

/// 有整句路径还没拿到神经分就请求并等到分回来；返回是否等到了（等到了调用方该重新查询）。
fn settle(engine: &mut Engine) -> bool {
    if !engine.rescoring_pending() || !engine.request_rescoring() {
        return false;
    }
    let started = Instant::now();
    while !engine.poll_rescoring() {
        if started.elapsed() > WAIT {
            tracing::warn!("等重打分超时");
            return false;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    true
}

/// 重排最多几轮：第一轮给前几条路径打分，第二轮给拼出来的那条打分，留一轮余量。
const MAX_ROUNDS: usize = 3;

/// 像壳一样把重排等完：分回来就重查，重查又有新的要打分（第二轮拼出来的路径）就再等，直到没有或满 [`MAX_ROUNDS`] 轮。
pub fn settled(engine: &mut Engine, query: Query) -> Query {
    let mut query = query;
    for _ in 0..MAX_ROUNDS {
        if !settle(engine) {
            break;
        }
        match engine.query() {
            Ok(again) => query = again,
            Err(_) => break,
        }
    }
    query
}
