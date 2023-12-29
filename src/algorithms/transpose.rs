/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::graph::labeled_arc_list_graph;
use crate::prelude::proj::Left;
use crate::prelude::{BitDeserializer, BitSerializer, LabelledSequentialGraph, SequentialGraph};
use crate::traits::graph::UnitLabelGraph;
use crate::utils::{BatchIterator, KMergeIters, SortPairs};
use anyhow::Result;
use dsi_progress_logger::*;
use lender::*;
use lender_derive::*;
/// Create transpose the graph and return a sequential graph view of it
#[allow(clippy::type_complexity)]
pub fn transpose_labelled<S: BitSerializer + Clone, D: BitDeserializer + Clone + 'static>(
    graph: &impl LabelledSequentialGraph<S::SerType>,
    batch_size: usize,
    serializer: S,
    deserializer: D,
) -> Result<labeled_arc_list_graph::LabeledArcListGraph<KMergeIters<BatchIterator<D>, D::DeserType>>>
where
    D::DeserType: Clone,
{
    let dir = tempfile::tempdir()?;
    let mut sorted = SortPairs::new_labeled(batch_size, dir.into_path(), serializer, deserializer)?;

    let mut pl = ProgressLogger::default();
    pl.item_name("node")
        .expected_updates(Some(graph.num_nodes()));
    pl.start("Creating batches...");
    // create batches of sorted edges
    for_!( (src, succ) in graph.iter() {
        for (dst, l) in succ {
            sorted.push_labeled(dst, src, l)?;
        }
        pl.light_update();
    });
    // merge the batches
    let sorted =
        labeled_arc_list_graph::LabeledArcListGraph::new(graph.num_nodes(), sorted.iter()?);
    pl.done();

    Ok(sorted)
}

pub fn transpose(
    graph: impl SequentialGraph,
    batch_size: usize,
) -> Result<Left<labeled_arc_list_graph::LabeledArcListGraph<KMergeIters<BatchIterator<()>, ()>>>> {
    Ok(Left(transpose_labelled(
        &UnitLabelGraph(graph),
        batch_size,
        (),
        (),
    )?))
}

/*
#[cfg(test)]
#[cfg_attr(test, test)]
fn test_transposition() -> anyhow::Result<()> {
    use crate::{
        graph::{arc_list_graph::ArcListGraph, vec_graph::VecGraph},
        prelude::UnitLabelGraph,
    };
    let arcs = vec![(0, 1), (0, 2), (1, 2), (1, 3), (2, 4), (3, 4)];
    let g = UnitLabelGraph(VecGraph::from_arc_list(&arcs));

    let trans = transpose(&g, 3, (), ())?;
    let g2 = UnitLabelGraph(VecGraph::from_node_iter::<&ArcListGraph<_>>(&trans));

    let trans = transpose(&g2, 3, (), ())?;
    let g3 = UnitLabelGraph(VecGraph::from_node_iter::<&ArcListGraph<_>>(&trans));

    assert_eq!(g, g3);
    Ok(())
}

#[cfg(test)]
#[cfg_attr(test, test)]
fn test_transposition_labeled() -> anyhow::Result<()> {
    use crate::graph::vec_graph::VecGraph;
    use dsi_bitstream::prelude::*;

    #[derive(Clone, Copy, PartialEq, Debug)]
    struct Payload(f64);

    impl Label for Payload {
        fn from_bitstream<E: Endianness, B: CodeRead<E>>(bitstream: &mut B) -> Result<Self> {
            let mantissa = bitstream.read_gamma()?;
            let exponent = bitstream.read_gamma()?;
            let result = f64::from_bits((exponent << 53) | mantissa);
            Ok(Payload(result))
        }

        fn to_bitstream<E: Endianness, B: CodeWrite<E>>(
            &self,
            bitstream: &mut B,
        ) -> Result<usize> {
            let value = self.0 as u64;
            let mantissa = value & ((1 << 53) - 1);
            let exponent = value >> 53;
            let mut written_bits = 0;
            written_bits += bitstream.write_gamma(mantissa)?;
            written_bits += bitstream.write_gamma(exponent)?;
            Ok(written_bits)
        }
    }
    let arcs = vec![
        (0, 1, Payload(1.0)),
        (0, 2, Payload(f64::EPSILON)),
        (1, 2, Payload(2.0)),
        (1, 3, Payload(f64::NAN)),
        (2, 4, Payload(f64::INFINITY)),
        (3, 4, Payload(f64::NEG_INFINITY)),
    ];

    // test transposition without labels
    let g = VecGraph::from_arc_and_label_list(&arcs);

    let trans = transpose(&g, 3)?;
    let g2 = VecGraph::from_node_iter(trans.iter_nodes());

    let trans = transpose(&g2, 3)?;
    let g3 = VecGraph::from_node_iter(trans.iter_nodes());

    let g4 = VecGraph::from_node_iter(g.iter_nodes());

    assert_eq!(g3, g4);

    //// test transposition with labels
    //let trans = transpose_labeled(&g, 3)?;
    //let g5 = VecGraph::from_labeled_node_iter(trans.iter_nodes());
    //
    //let trans = transpose_labeled(&g5, 3)?;
    //let g6 = VecGraph::from_labeled_node_iter(trans.iter_nodes());
    //
    //assert_eq!(g, g6);
    Ok(())
}
*/
