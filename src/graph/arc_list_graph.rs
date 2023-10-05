/*
 * SPDX-FileCopyrightText: 2023 Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use crate::traits::*;

/// An adapter exhibiting a list of arcs sorted by source as a [sequential graph](SequentialGraph).
///
/// If for every source the arcs are sorted by destination, the
/// successors of the graph will be sorted.
#[derive(Debug, Clone)]
pub struct ArcListGraph<I: Clone> {
    num_nodes: usize,
    into_iter: I,
}

impl<I: IntoIterator<Item = (usize, usize)> + Clone> ArcListGraph<I> {
    /// Create a new graph from an iterator of pairs of nodes
    pub fn new(num_nodes: usize, iter: I) -> Self {
        Self {
            num_nodes,
            into_iter: iter,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeIterator<I: Iterator<Item = (usize, usize)>> {
    num_nodes: usize,
    curr_node: usize,
    next_pair: (usize, usize),
    iter: I,
}

unsafe impl<I: Iterator<Item = (usize, usize)> + Clone> SortedIterator
    for NodeIterator<I>
{
}

impl<I: Iterator<Item = (usize, usize)>> NodeIterator<I> {
    pub fn new(num_nodes: usize, mut iter: I) -> Self {
        NodeIterator {
            num_nodes,
            curr_node: 0_usize.wrapping_sub(1), // No node seen yet
            next_pair: iter.next().unwrap_or((usize::MAX, usize::MAX)),
            iter,
        }
    }
}

impl<'succ, I: Iterator<Item = (usize, usize)> + Clone> LendingIteratorItem<'succ>
    for NodeIterator<I>
{
    type T = (usize, Successors<'succ, I>);
}

impl<I: Iterator<Item = (usize, usize)> + Clone> LendingIterator for NodeIterator<I> {
    fn next(&mut self) -> Option<(usize, Successors<'_, I>)> {
        self.curr_node = self.curr_node.wrapping_add(1);
        if self.curr_node == self.num_nodes {
            return None;
        }

        // This happens if the user doesn't use the successors iter
        while self.next_pair.0 < self.curr_node {
            self.next_pair = self.iter.next().unwrap_or((usize::MAX, usize::MAX));
        }

        Some((self.curr_node, Successors { node_iter: self }))
    }
}

impl<I: IntoIterator<Item = (usize, usize)> + Clone + 'static> SequentialGraph for ArcListGraph<I>
where
    I::IntoIter: Clone,
{
    type Successors<'succ> = Successors<'succ, I::IntoIter>
    where
        Self: 'succ,
        I: 'succ;
    type Iterator<'node> = NodeIterator<I::IntoIter>
    where Self: 'node,
        I: 'node;

    #[inline(always)]
    fn num_nodes(&self) -> usize {
        self.num_nodes
    }

    #[inline(always)]
    fn num_arcs_hint(&self) -> Option<usize> {
        None
    }

    /// Get an iterator over the nodes of the graph
    fn iter_from(&self, from: usize) -> NodeIterator<I::IntoIter> {
        let mut iter = NodeIterator::new(self.num_nodes, self.into_iter.clone().into_iter());
        for _ in 0..from {
            iter.next();
        }
        iter
    }
}

pub struct Successors<'succ, I: Iterator<Item = (usize, usize)>> {
    node_iter: &'succ mut NodeIterator<I>,
}

impl<'succ, I: Iterator<Item = (usize, usize)>> Iterator for Successors<'succ, I> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        // if we reached a new node, the successors of curr_node are finished
        if self.node_iter.next_pair.0 != self.node_iter.curr_node {
            None
        } else {
            // get the next triple
            let pair = self
                .node_iter
                .iter
                .next()
                .unwrap_or((usize::MAX, usize::MAX));
            // store the triple and return the previous successor
            // storing the label since it should be one step behind the successor
            let (_src, dst) = core::mem::replace(&mut self.node_iter.next_pair, pair);
            Some(dst)
        }
    }
}

#[cfg(test)]
#[cfg_attr(test, test)]
fn test_coo_iter() -> anyhow::Result<()> {
    use crate::graph::vec_graph::VecGraph;
    let arcs = vec![(0, 1), (0, 2), (1, 2), (1, 3), (2, 4), (3, 4)];
    let g = VecGraph::from_arc_list(&arcs);
    let coo = ArcListGraph::new(g.num_nodes(), arcs);
    let g2 = VecGraph::from_node_iter::<NodeIterator<_>>(coo.iter());
    assert_eq!(g, g2);
    Ok(())
}
