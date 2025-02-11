#![cfg(test)]
use futures::future::try_join_all;
use libp2p::{self, Multiaddr};
use noosphere_core::authority::generate_ed25519_key;
use noosphere_ns::dht::{DHTConfig, DHTError, DHTNode, RecordValidator};
use rand::{thread_rng, Rng};
use std::future::Future;
use std::time::Duration;

fn generate_listening_addr() -> Multiaddr {
    format!(
        "/ip4/127.0.0.1/tcp/{}",
        thread_rng().gen_range(49152..65535)
    )
    .parse()
    .expect("parseable")
}

pub async fn wait_ms(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

async fn await_or_timeout<T>(
    timeout_ms: u64,
    future: impl Future<Output = T>,
    message: String,
) -> T {
    tokio::select! {
        _ = wait_ms(timeout_ms) => { panic!("timed out: {}", message); }
        result = future => { result }
    }
}

pub fn create_test_config() -> DHTConfig {
    DHTConfig {
        listening_address: Some(generate_listening_addr()),
        peer_dialing_interval: 1,
        ..Default::default()
    }
}

pub async fn swarm_command<'a, V: RecordValidator, TFuture, F, T, E>(
    nodes: &'a mut [DHTNode<V>],
    func: F,
) -> Result<Vec<T>, E>
where
    F: FnMut(&'a mut DHTNode<V>) -> TFuture,
    TFuture: Future<Output = Result<T, E>>,
{
    let futures: Vec<_> = nodes.iter_mut().map(func).collect();
    try_join_all(futures).await
}

pub fn create_nodes_with_bootstrap<V: RecordValidator + Clone, W: RecordValidator>(
    client_count: usize,
    bootstrap_nodes: &[DHTNode<W>],
    validator: V,
) -> Result<Vec<DHTNode<V>>, DHTError> {
    let bootstrap_addresses: Vec<Multiaddr> = bootstrap_nodes
        .iter()
        .map(|node| node.p2p_address().unwrap().to_owned())
        .collect();

    let mut client_nodes: Vec<DHTNode<V>> = vec![];
    for _ in 0..client_count {
        let key_material = generate_ed25519_key();
        let config = create_test_config();
        let mut node: DHTNode<V> = DHTNode::new(
            &key_material,
            Some(&bootstrap_addresses),
            validator.clone(),
            &config,
        )?;
        node.run()?;
        client_nodes.push(node);
    }
    Ok(client_nodes)
}

/// Creates `count` bootstrap nodes, each node using all other
/// bootstrap nodes as bootstrap peers.
pub fn create_bootstrap_nodes<V: RecordValidator + Clone>(
    count: usize,
    validator: V,
) -> Result<Vec<DHTNode<V>>, DHTError> {
    let mut nodes: Vec<DHTNode<V>> = vec![];
    let mut addresses: Vec<Multiaddr> = vec![];
    for _ in 0..count {
        let key_material = generate_ed25519_key();
        let config = create_test_config();
        let node: DHTNode<V> = DHTNode::new(&key_material, None, validator.clone(), &config)?;
        addresses.push(node.p2p_address().unwrap().to_owned());
        nodes.push(node);
    }

    for (i, node) in nodes.iter_mut().enumerate() {
        let mut peers = addresses.clone();
        // Remove a node's own address from peers
        peers.remove(i);
        node.add_peers(&peers)?;
        node.run()?;
    }
    Ok(nodes)
}

pub async fn initialize_network<V: RecordValidator + Clone + 'static>(
    bootstrap_count: usize,
    client_count: usize,
    validator: V,
) -> Result<(Vec<DHTNode<V>>, Vec<DHTNode<V>>), DHTError> {
    let mut bootstrap_nodes = create_bootstrap_nodes::<V>(bootstrap_count, validator.clone())?;
    let mut client_nodes =
        create_nodes_with_bootstrap::<V, V>(client_count, &bootstrap_nodes, validator.clone())?;
    let expected_peers = client_count + bootstrap_count - 1;
    // Wait a few, since nodes need to announce each other via Identify,
    // which adds their address to the routing table. Kick off
    // another bootstrap process after that.
    // @TODO Figure out if bootstrapping is needed after identify-exchange,
    // as that typically happens on a ~5 minute timer.
    wait_ms(700).await;
    swarm_command(&mut client_nodes, |c| c.bootstrap()).await?;

    // Wait for the peers to establish connections.
    await_or_timeout(
        2000,
        swarm_command(&mut client_nodes, |c| c.wait_for_peers(expected_peers)),
        format!("waiting for {} peers", expected_peers),
    )
    .await?;

    await_or_timeout(
        2000,
        swarm_command(&mut bootstrap_nodes, |c| c.wait_for_peers(expected_peers)),
        format!("waiting for {} peers", expected_peers),
    )
    .await?;
    Ok((bootstrap_nodes, client_nodes))
}
