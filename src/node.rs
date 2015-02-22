use collect::compare::Compare;
use std::cmp::Ordering::*;
use std::mem;

pub type Link<K, V> = Option<Box<Node<K, V>>>;

pub trait LinkExt: Sized {
    type K;
    type V;
    fn key_value(&self) -> Option<(&Self::K, &Self::V)>;
    fn key_value_mut(&mut self) -> Option<(&Self::K, &mut Self::V)>;
}

impl<K, V> LinkExt for Link<K, V> {
    type K = K;
    type V = V;

    fn key_value(&self) -> Option<(&K, &V)> {
        self.as_ref().map(|node| (&node.key, &node.value))
    }

    fn key_value_mut(&mut self) -> Option<(&K, &mut V)> {
        self.as_mut().map(|&mut box ref mut node| (&node.key, &mut node.value))
    }
}

#[derive(Clone)]
pub struct Node<K, V> {
    left: Link<K, V>,
    right: Link<K, V>,
    key: K,
    value: V,
}

pub fn insert<K, V, C>(link: &mut Link<K, V>, cmp: &C, key: K, value: V) -> Option<V>
    where C: Compare<K> {

    match *link {
        None => {
            *link = Some(box Node { left: None, right: None, key: key, value: value });
            None
        }
        Some(ref mut node) => match cmp.compare(&key, &node.key) {
            Equal => {
                node.key = key;
                Some(mem::replace(&mut node.value, value))
            }
            Less => insert(&mut node.left, cmp, key, value),
            Greater => insert(&mut node.right, cmp, key, value),
        },
    }
}

trait LinkRef<'a>: Sized {
    type K: 'a;
    type V: 'a;
    fn as_ref(self) -> &'a Link<Self::K, Self::V>;
    unsafe fn from_ref(link: &'a Link<Self::K, Self::V>) -> Self;

    fn with<F>(self, f: F) -> Self
        where F: FnOnce(&'a Link<Self::K, Self::V>) -> &'a Link<Self::K, Self::V> {

        let link = f(self.as_ref());
        unsafe { LinkRef::from_ref(link) }
    }
}

impl<'a, K: 'a, V: 'a> LinkRef<'a> for &'a Link<K, V> {
    type K = K;
    type V = V;

    fn as_ref(self) -> &'a Link<K, V> { self }

    fn from_ref(link: &'a Link<K, V>) -> &'a Link<K, V> { link }
}

impl<'a, K: 'a, V: 'a> LinkRef<'a> for &'a mut Link<K, V> {
    type K = K;
    type V = V;

    fn as_ref(self) -> &'a Link<K, V> { self }

    unsafe fn from_ref(link: &'a Link<K, V>) -> &'a mut Link<K, V> {
        mem::transmute(link)
    }
}

pub fn get<'a, L: LinkRef<'a>, C, Q: ?Sized>(link: L, cmp: &C, key: &Q) -> L
    where C: Compare<Q, L::K> {

    link.with(|mut link| loop {
        match *link {
            None => return link,
            Some(ref node) => match cmp.compare(key, &node.key) {
                Equal => return link,
                Less => link = &node.left,
                Greater => link = &node.right,
            },
        }
    })
}

trait Dir: ::std::marker::MarkerTrait {
    fn forward<L, R, T>(left: L, right: R) -> T
        where L: FnOnce() -> T, R: FnOnce() -> T;

    fn reverse<L, R, T>(left: L, right: R) -> T
        where L: FnOnce() -> T, R: FnOnce() -> T;
}

pub enum Left {}

impl Dir for Left {
    fn forward<L, R, T>(left: L, _right: R) -> T
        where L: FnOnce() -> T, R: FnOnce() -> T { left() }

    fn reverse<L, R, T>(_left: L, right: R) -> T
        where L: FnOnce() -> T, R: FnOnce() -> T { right() }
}

pub enum Right {}

impl Dir for Right {
    fn forward<L, R, T>(_left: L, right: R) -> T
        where L: FnOnce() -> T, R: FnOnce() -> T { right() }

    fn reverse<L, R, T>(left: L, _right: R) -> T
        where L: FnOnce() -> T, R: FnOnce() -> T { left() }
}

pub fn extremum<'a, L, D>(link: L) -> L where L: LinkRef<'a>, D: Dir {
    link.with(|mut link| {
        while let Some(ref node) = *link {
            let child = <D as Dir>::forward(|| &node.left, || &node.right);
            if child.is_some() { link = child; } else { break; }
        }

        link
    })
}
