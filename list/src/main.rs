struct LinkedList {
    head: Option<Box<Node>>,
    size: usize,
}

struct Node {
    value: u32,
    next: Option<Box<Node>>,
}

impl Node {
    pub fn new(value: u32, next: Option<Box<Node>>) -> Node {
        Node {
            value: value,
            next: next,
        }
    }
}

impl LinkedList {
    pub fn new() -> LinkedList {
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

    pub fn push(&mut self, value: u32) {
        let new_node = Box::new(Node::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }

    pub fn pop(&mut self) -> Option<u32> {
        let head = self.head.take()?;
        self.head = head.next;
        self.size -= 1;
        Some(head.value)
    }

    pub fn display(&self) {
        let mut current_node = &self.head;
        while let Some(node) = current_node {
            print!("{} ", node.value);
            current_node = &node.next;
        }
        println!(" ")
    }
}

fn main() {
    let mut list: LinkedList = LinkedList::new();
    for i in 0..10 {
        list.push(i);
    }
    list.display();
    list.pop();
    list.display();
    println!("{}", list.get_size());
}
