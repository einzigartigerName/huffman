extern crate bitwise;

use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::path::{PathBuf};
use std::io::{BufRead, Write, BufReader, BufWriter, Error, ErrorKind};
use std::collections::HashMap;
use std::cmp::Ordering;
use std::u8::MAX;
use bitwise::*;

#[derive(Debug, Eq, Hash)]
enum TLeaf {
    Character(Occur),
    EOF,
}

#[derive(Eq, Hash)]
struct Occur {
    value: u8,
    count: u64,
}

#[derive(Debug, Eq, Hash)]
struct Node {
    count: u64,
    l: Box<TNode>,
    r: Box<TNode>,
}

#[derive(Debug, Eq, Hash)]
enum TNode {
    Leaf(TLeaf),
    Inner(Node),
}

const BUFFER_SIZE: usize = 256;
const EOF_VALUE: u16 = 0xFFFF;


/**************************************************************************************************
                        Occur - Implementations
 *************************************************************************************************/
impl Ord for Occur {
    fn cmp(&self, other: &Self) -> Ordering {
        other.count.cmp(&self.count)
    }
}

impl PartialOrd for Occur {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Occur {
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count
    }
}

impl Debug for Occur {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Byte: {} - {} - {}", self.value as char, self.value, self.count)
    }
}


/**************************************************************************************************
                        Node - Implementations
 *************************************************************************************************/
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.count.cmp(&self.count)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count
    }
}


/**************************************************************************************************
                        TNode - Implementations
 *************************************************************************************************/
impl Ord for TNode {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = self.get_count();
        let b = other.get_count();

        b.cmp(&a)
    }
}

impl PartialOrd for TNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for TNode {
    fn eq(&self, other: &Self) -> bool {
        let a = self.get_count();
        let b = other.get_count();

        a == b
    }
}

impl TNode {
    fn get_count(&self) -> u64 {
        match self {
            TNode::Leaf(l) => match l {
                TLeaf::EOF => 0,
                TLeaf::Character(occ) => occ.count,
            }
            TNode::Inner(node) => node.count,
        }
    }
}

/**************************************************************************************************
                        TLeaf - Implementations
 *************************************************************************************************/
impl PartialEq for TLeaf {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}


/**************************************************************************************************
                        Main Methode Implementations
 *************************************************************************************************/
///
/// Encode the Input with the given Lookup-Table
///
pub fn encode(input: &PathBuf) -> std::io::Result<PathBuf> {
    /* Create output Path */
    let mut path = String::new();
    path.push_str(&input.to_str().unwrap());
    path.push_str(".huff");

    let to = PathBuf::from(path);

    /* First Pass - Count Occurs */
    let counts = analyse(&input)?;

    let file_in = File::open(&input)?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file_in);

    /* Build Lookup Table to Encode */
    let mut tree = construct_tree(counts).unwrap();

    /* Writer for encoded Data */
    let file_out = File::create(&to)?;
    let mut writer = BitWriter::new(file_out, true);

    /* Encode tree */
    encode_tree(&mut writer, &tree);

    /* Append EOF and create Lookup */
    tree_append_eof(&mut tree);
    let mut map: HashMap<u16, Vec<Bit>> = HashMap::new();
    create_lookup(&tree, Vec::new(), &mut map);

    /* Read File and encode byte by byte */
    loop {
        let buffer = reader.fill_buf().unwrap();

        // if buffer empty -> eof
        if buffer.is_empty() {
            break;
        }

        for byte in buffer {
            writer.write_bits(&map.get(&(*byte as u16)).unwrap())?;
        }

        // Consume read bytes
        let length = buffer.len();
        reader.consume(length);
    }

    /* Add EOF */
    writer.write_bits(map.get(&EOF_VALUE).unwrap())?;

    /* Write the remaining buffer in Writer to file */
    writer.write_buf()?;

    Ok(to)
}

///
/// Decode a File and Write Result
///
pub fn decode(input: &mut PathBuf) -> std::io::Result<PathBuf> {
    /* Create output Path */
    let to = PathBuf::from(&input).with_extension("");

    /* BitReader */
    let file_in = File::open(&input)?;
    let mut reader = BitReader::new(BufReader::new(file_in))?;

    /* decode tree and append EOF sequence */
    let mut tree = decode_tree(&mut reader)?;
    tree_append_eof(&mut tree);

    /* Create Output File */
    let file_out = File::create(&to)?;
    let mut writer = BufWriter::new(file_out);

    let mut current = &tree;
    let mut counter: u8 = 0;

    /* Decode File */
    loop {
        // if reader empty -> eof
        if reader.is_empty() {
            break;
        }

        /* If in Leaf write result */
        if let TNode::Leaf(l) = current {
            match l {
                // EOF -> Finish Decoding
                TLeaf::EOF => {
                    return Ok(to)
                },
                // Character -> Write to file and reset current
                TLeaf::Character(occ) => {
                    writer.write(&[occ.value]).unwrap();
                    counter += 1;

                    if counter == MAX {
                        writer.flush().unwrap();
                        counter = 0;
                    }

                    current = &tree;
                }
            }
        }

        /* Read next bit */
        let bit = reader.read().unwrap();

        /* Walk Tree */
        match current {
            TNode::Leaf(_) => {}
            TNode::Inner(node) => {
                match bit {
                    Bit::Zero => {
                        current = &*node.l;
                    }
                    Bit::One => {
                        current = &*node.r;
                    }
                }
            }
        };
    }

    /* if premature eof and decoded byte in buffer -> write it */
    if let TNode::Leaf(l) = current {
        match l {
            TLeaf::EOF => {
                return Ok(to);
            },
            TLeaf::Character(occ) => {
                let _ = writer.write(&[occ.value]);
            }
        }
    }

    /* Tell user about premature EOF */
    Err(err_eof())
}


/**************************************************************************************************
                        Help Methode Implementations
 *************************************************************************************************/
///
/// Bytewise analyse the given file and return result vector which counted occurrences
///
fn analyse(input: &PathBuf) -> std::io::Result<[u64; 256]> {
    let file_in = File::open(input)?;
    let mut reader = BufReader::new(file_in);

    let mut counter = [0 as u64; 256];

    loop {
        let buffer = reader.fill_buf().unwrap();

        // work with buffer
        if buffer.is_empty() {
            break;
        }

        for byte in buffer {
            counter[(*byte) as usize] += 1;
        }

        // ensure the bytes we worked with aren't returned again later
        let length = buffer.len();
        reader.consume(length);
    }

    Ok(counter)
}

///
/// Create a lookup Table from a Huffman Tree
///
fn create_lookup(tree: &TNode, mut bin: Vec<Bit>, table: &mut HashMap<u16, Vec<Bit>>) {
    match tree {
        TNode::Leaf(l) => match l {
            TLeaf::EOF => {
                table.insert(EOF_VALUE, bin);
            },
            TLeaf::Character(occ) => {
                table.insert(occ.value as u16, bin);
            },
        }
        TNode::Inner(node) => {
            let mut bin_l = bin.to_vec();
            bin_l.push(Bit::Zero);
            bin.push(Bit::One);
            create_lookup(&*node.l, bin_l, table);
            create_lookup(&*node.r, bin, table);
        }
    }
}

///
/// Encode the Huffman Tree to a Bit Representation
///
fn encode_tree<W: Write>(writer: &mut BitWriter<W>, tree: &TNode) {
    match tree {
        TNode::Leaf(l) => match l {
            TLeaf::EOF => {},
            TLeaf::Character(occ) => {
                let _ = writer.write(Bit::One);
                let _ = writer.write_bits(&byte_to_bits(occ.value));
            }
        }
        TNode::Inner(node) => {
            let _ = writer.write(Bit::Zero);
            encode_tree(writer, &*node.l);
            encode_tree(writer, &*node.r);
        }
    }
}

///
/// Decodes and builds the Huffman Tree
/// returns (Consumed Bits, Huffman Tree)
///
fn decode_tree<R: BufRead>(reader: &mut BitReader<R>) -> std::io::Result<TNode> {
    if reader.is_empty() {
        return Err(err_eof())
    }

    match reader.read().unwrap() {
        Bit::Zero => {
            let left = decode_tree(reader)?;
            let right = decode_tree(reader)?;

            Ok(
                TNode::Inner(
                    Node {
                        count: 0,
                        l: Box::from(left),
                        r: Box::from(right),
                    }
                )
            )
        }
        Bit::One => {
            match bits_to_byte(&*reader.read_multi(8)?) {
                Some(c) => Ok(
                    TNode::Leaf(
                        TLeaf::Character(
                            Occur { value: c, count: 0 }
                        )
                    )
                ),
                None => Err(err_bits_to_byte()),
            }
        }
    }
}

///
/// Creates a Huffman Tree
/// * Left -> Add 0
/// * Right -> Add 1
///
fn construct_tree(counts: [u64; 256]) -> Option<TNode> {
    let mut queue = Vec::with_capacity(counts.len());

    for i in 0..counts.len() {
        if counts[i] != 0 {
            queue.push(
                TNode::Leaf(
                    TLeaf::Character(
                        Occur {
                            value: i as u8,
                            count: counts[i]
                        }
                    )
                )
            )
        }
    }

    loop {
        queue.sort();

        if queue.len() <= 1 {
            break;
        }

        let l = queue.pop().unwrap();
        let r = queue.pop().unwrap();

        let combined = TNode::Inner(
            Node {
                count: l.get_count() + r.get_count(),
                l: Box::from(l),
                r: Box::from(r),
            }
        );

        queue.push(combined);
    }

    queue.pop()
}

///
/// Appends a EOF sequence to the tree
///
fn tree_append_eof(tree: &mut TNode) {
    let (_,leaf) = tree_find_deepest(tree, 0);

    if let TNode::Leaf(TLeaf::Character(occ)) = &leaf {
        let val = occ.value;
        let c = occ.count;

        *leaf = TNode::Inner(
            Node {
                count: c,
                l: Box::new(
                    TNode::Leaf(
                        TLeaf::Character(
                            Occur { value: val, count: c }
                        )
                    )
                ),
                r: Box::new(
                    TNode::Leaf(
                        TLeaf::EOF
                    )
                ),
            }
        );
    }
}

///
/// Finds the deepest Leaf in a Tree
///
fn tree_find_deepest(tree: &mut TNode, depth: usize) -> (usize, &mut TNode) {
    return match tree {
        TNode::Inner(node) => {
            let (depth_l, node_l) = tree_find_deepest(&mut *node.l, depth + 1);
            let (depth_r, node_r) = tree_find_deepest(&mut *node.r, depth + 1);

            if depth_r >= depth_l {
                (depth_r, node_r)
            } else {
                (depth_l, node_l)
            }
        },
        TNode::Leaf(_) => {
            (depth, tree)
        },
    }
}

/**************************************************************************************************
                        Utility Methode
 *************************************************************************************************/
fn bits_to_byte(bits: &[Bit]) -> Option<u8> {
    if bits.len() != 8 {
        return None;
    }

    let mut output = 0 as u8;

    for i in 0..8 {
        output |= bits[i] as u8;

        if i < 7 {
            output = output << 1;
        }
    }

    Some(output)
}

fn byte_to_bits(byte: u8) -> Vec<Bit> {
    let mut bits = Vec::new();

    for i in (0..8).rev() {
        let mask = 1 << i;

        if byte & mask > 0 {
            bits.push(Bit::One)
        } else {
            bits.push(Bit::Zero)
        }
    }

    bits
}

fn err_eof() -> Error {
    Error::new(ErrorKind::Other, "Premature EOF")
}

fn err_bits_to_byte() -> Error {
    Error::new(ErrorKind::Other, "Bundling Bits to Byte")
}
