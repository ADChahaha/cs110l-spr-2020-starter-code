use core::fmt;
use std::str::FromStr;

struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new(value: T, next: Option<Box<Node<T>>>) -> Node<T> {
        Node {
            value: value,
            next: next,
        }
    }
}

impl<T> LinkedList<T> {
    pub fn new() -> LinkedList<T> {
        LinkedList {
            head: None,
            size: 0,
        }
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn push(&mut self, value: T) {
        let new_node = Box::new(Node::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        let head = self.head.take()?;
        self.head = head.next;
        self.size -= 1;
        Some(head.value)
    }
}
impl<T> LinkedList<T>
where
    T: fmt::Display,
{
    pub fn display(&self) {
        let mut current_node = &self.head;
        while let Some(node) = current_node {
            print!("{} ", node.value);
            current_node = &node.next;
        }
        println!(" ")
    }
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut current = &self.head;
        let mut result = String::new();
        loop {
            match current {
                Some(node) => {
                    result = format!("{} {}", result, node.value);
                    current = &node.next;
                }
                None => break,
            }
        }
        write!(f, "{}", result)
    }
}
impl<T> Clone for LinkedList<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        let mut new_list = Self {
            head: None,
            size: 0,
        };
        let mut arr: Vec<T> = Vec::new();
        let mut current = &self.head;
        while let Some(node) = current {
            arr.push(node.value.clone());
            current = &node.next;
        }
        while let Some(value) = arr.pop() {
            new_list.push(value);
        }
        new_list
    }
}

impl<T> PartialEq for LinkedList<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if self.size != other.size {
            return false;
        }
        let mut current_node1 = &self.head;
        let mut current_node2 = &other.head;
        while let Some(node1) = current_node1
            && let Some(node2) = current_node2
        {
            if node1.value != node2.value {
                return false;
            }
            current_node1 = &node1.next;
            current_node2 = &node2.next;
        }
        true
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}

fn main() {
    let mut list: LinkedList<String> = LinkedList::new();
    list.push(String::from_str("jdj").unwrap());
    list.push(String::from_str("jdj").unwrap());
    let list2 = list.clone();
    list.pop();
    list.display();
    list2.display();
}
