#[cfg(feature = "range")] use compare::Compare;
#[cfg(feature = "range")] use std::cmp::Ordering::*;
#[cfg(feature = "range")] use std::collections::Bound;
use std::collections::VecDeque;
use std::marker::PhantomData;
use super::Node;

pub trait NodeRef: Sized {
    type Key;
    type Item;
    fn key(&self) -> &Self::Key;
    fn item(self) -> Self::Item;
    fn left(&mut self) -> Option<Self>;
    fn right(&mut self) -> Option<Self>;
}

pub struct MarkedNode<'a, K: 'a, V: 'a> {
    node: &'a Node<K, V>,
    seen_l: bool,
    seen_r: bool,
}

impl<'a, K, V> Clone for MarkedNode<'a, K, V> {
    fn clone(&self) -> Self { *self }
}

impl<'a, K, V> Copy for MarkedNode<'a, K, V> {}

impl<'a, K, V> MarkedNode<'a, K, V> {
    pub fn new(node: &'a Box<Node<K, V>>) -> Self {
        MarkedNode { node: node, seen_l: false, seen_r: false }
    }
}

impl<'a, K, V> NodeRef for MarkedNode<'a, K, V> {
    type Key = K;
    type Item = (&'a K, &'a V);
    fn key(&self) -> &Self::Key { &self.node.key }
    fn item(self) -> Self::Item { (&self.node.key, &self.node.value) }

    fn left(&mut self) -> Option<Self> {
        if self.seen_l {
            None
        } else {
            self.seen_l = true;
            self.node.left.as_ref().map(MarkedNode::new)
        }
    }

    fn right(&mut self) -> Option<Self> {
        if self.seen_r {
            None
        } else {
            self.seen_r = true;
            self.node.right.as_ref().map(MarkedNode::new)
        }
    }
}

pub struct MutMarkedNode<'a, K: 'a, V: 'a> {
    node: *mut Node<K, V>,
    seen_l: bool,
    seen_r: bool,
    _marker: PhantomData<&'a mut Node<K, V>>,
}

impl<'a, K, V> MutMarkedNode<'a, K, V> {
    pub fn new(node: &'a mut Box<Node<K, V>>) -> Self {
        MutMarkedNode { node: &mut **node, seen_l: false, seen_r: false, _marker: PhantomData }
    }
}

unsafe impl<'a, K, V> Send for MutMarkedNode<'a, K, V> where K: Send, V: Send {}
unsafe impl<'a, K, V> Sync for MutMarkedNode<'a, K, V> where K: Sync, V: Sync {}

impl<'a, K, V> NodeRef for MutMarkedNode<'a, K, V> {
    type Key = K;
    type Item = (&'a K, &'a mut V);

    fn key(&self) -> &Self::Key { &unsafe { &*self.node}.key }

    fn item(self) -> Self::Item {
        let node = unsafe { &mut *self.node };
        (&node.key, &mut node.value)
    }

    fn left(&mut self) -> Option<Self> {
        if self.seen_l {
            None
        } else {
            self.seen_l = true;
            unsafe { &mut *self.node}.left.as_mut().map(MutMarkedNode::new)
        }
    }

    fn right(&mut self) -> Option<Self> {
        if self.seen_r {
            None
        } else {
            self.seen_r = true;
            unsafe { &mut *self.node}.right.as_mut().map(MutMarkedNode::new)
        }
    }
}

impl<K, V> NodeRef for Box<Node<K, V>> {
    type Key = K;
    type Item = (K, V);
    fn key(&self) -> &Self::Key { &self.key }
    fn item(self) -> Self::Item { let node = *self; (node.key, node.value) }
    fn left(&mut self) -> Option<Self> { self.left.take() }
    fn right(&mut self) -> Option<Self> { self.right.take() }
}

#[derive(Clone)]
pub struct Iter<N> where N: NodeRef {
    nodes: VecDeque<N>,
    size: usize,
}

impl<N> Iter<N> where N: NodeRef {
    pub fn new(root: Option<N>, size: usize) -> Self {
        Iter { nodes: root.into_iter().collect(), size: size }
    }
}

impl<N> Iterator for Iter<N> where N: NodeRef {
    type Item = N::Item;

    fn next(&mut self) -> Option<N::Item> {
        loop {
            let push = self.nodes.back_mut().and_then(N::left);

            match push {
                None => return self.nodes.pop_back().map(|mut node| {
                    self.size -= 1;
                    if let Some(right) = node.right() { self.nodes.push_back(right); }
                    node.item()
                }),
                Some(left) => self.nodes.push_back(left),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) { (self.size, Some(self.size)) }
}

impl<N> DoubleEndedIterator for Iter<N> where N: NodeRef {
    fn next_back(&mut self) -> Option<N::Item> {
        loop {
            let push = self.nodes.front_mut().and_then(N::right);

            match push {
                None => return self.nodes.pop_front().map(|mut node| {
                    self.size -= 1;
                    if let Some(left) = node.left() { self.nodes.push_front(left); }
                    node.item()
                }),
                Some(right) => self.nodes.push_front(right),
            }
        }
    }
}

impl<N> ExactSizeIterator for Iter<N> where N: NodeRef {
    fn len(&self) -> usize { self.size }
}

#[cfg(feature = "range")]
#[derive(Clone)]
pub struct Range<N>(Iter<N>) where N: NodeRef;

#[cfg(feature = "range")]
impl<N> Range<N> where N: NodeRef {
    pub fn new<C, Min: ?Sized, Max: ?Sized>(root: Option<N>, size: usize, cmp: &C,
                                            min: Bound<&Min>, max: Bound<&Max>) -> Self
        where C: Compare<Min, N::Key> + Compare<Max, N::Key> {

        fn bound_to_opt<T>(bound: Bound<T>) -> Option<(T, bool)> {
            match bound {
                Bound::Unbounded => None,
                Bound::Included(bound) => Some((bound, true)),
                Bound::Excluded(bound) => Some((bound, false)),
            }
        }

        enum Op<T> {
            PopPush(Option<T>, bool),
            Push(Option<T>),
        }

        let mut it = Iter::new(root, size);

        bound!(it, cmp, min, Less, Greater, left, right, back_mut, pop_back, push_back);
        bound!(it, cmp, max, Greater, Less, right, left, front_mut, pop_front, push_front);

        Range(it)
    }
}

#[cfg(feature = "range")]
impl<N> Iterator for Range<N> where N: NodeRef {
    type Item = N::Item;
    fn next(&mut self) -> Option<N::Item> { self.0.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { (self.0.nodes.len(), Some(self.0.size)) }
}

#[cfg(feature = "range")]
impl<N> DoubleEndedIterator for Range<N> where N: NodeRef {
    fn next_back(&mut self) -> Option<N::Item> { self.0.next_back() }
}
