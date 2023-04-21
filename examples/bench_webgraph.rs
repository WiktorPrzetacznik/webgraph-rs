use webgraph::prelude::*;

type ReadType = u32;
type BufferType = u64;

const NODES: usize = 325_557;
const ARCS: usize = 3_216_152;
const PATH: &str = "tests/data/cnr-2000";

pub fn main() {
    // Read the offsets
    let mut data = std::fs::read(format!("{}.offsets", PATH)).unwrap();
    // pad with zeros so we can read with ReadType words
    while data.len() % core::mem::size_of::<ReadType>() != 0 {
        data.push(0);
    }
    // we must do this becasue Vec<u8> is not guaranteed to be properly aligned
    let data = data
        .chunks(core::mem::size_of::<ReadType>())
        .map(|chunk| ReadType::from_ne_bytes(chunk.try_into().unwrap()))
        .collect::<Vec<_>>();

    // Read the offsets gammas
    let mut offsets = Vec::with_capacity(NODES);
    let mut reader = BufferedBitStreamRead::<M2L, BufferType, _>::new(MemWordReadInfinite::new(&data));
    let mut offset = 0;
    for _ in 0..NODES {
        offset += reader.read_gamma::<true>().unwrap() as usize;
        offsets.push(offset);
    }

    let mut data = std::fs::read(format!("{}.graph", PATH)).unwrap();
    // pad with zeros so we can read with ReadType words
    while data.len() % core::mem::size_of::<ReadType>() != 0 {
        data.push(0);
    }
    // we must do this becasue Vec<u8> is not guaranteed to be properly aligned
    let data = data
        .chunks(core::mem::size_of::<ReadType>())
        .map(|chunk| ReadType::from_ne_bytes(chunk.try_into().unwrap()))
        .collect::<Vec<_>>();

    // create a random access reader
    let code_reader = DefaultCodesReader::new(
        BufferedBitStreamRead::<M2L, BufferType, _>::new(MemWordReadInfinite::new(&data)),
    );
    let random_reader = WebgraphReaderRandomAccess::new(code_reader, offsets, 4);

    // Create a sequential reader
    let mut code_reader = DefaultCodesReader::new(
        BufferedBitStreamRead::<M2L, BufferType, _>::new(MemWordReadInfinite::new(&data)),
    );
    let mut seq_reader = WebgraphReaderSequential::new(&mut code_reader, 4, 16);

    // Check that they read the same
    let start = std::time::Instant::now();
    for node_id in 0..(NODES as u64) {
        let _ = seq_reader.get_successors_iter(node_id).unwrap();
    }
    println!("Sequential:{:>20} arcs/s", ARCS as f64 / start.elapsed().as_secs_f64());


    // Check that they read the same
    let start = std::time::Instant::now();
    for node_id in 0..(NODES as u64) {
        let _ = random_reader.get_successors_iter(node_id).unwrap().collect::<Vec<_>>();
    }
    println!("Random:    {:>20} arcs/s", ARCS as f64 / start.elapsed().as_secs_f64());
}