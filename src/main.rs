use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use quote::quote;
use syn::{
    visit::{self, Visit},
    Expr, File as SynFile, ItemFn, Stmt, Block,
};
use std::fs;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Clone)]
enum CfgNode {
    Function(String),
    Precondition(String),
    Postcondition(String),
    Invariant(String),
    Statement(String),
    Condition(String),
    Return(String),
    MergePoint,
    // FunctionCall can be included under Statement if desired
}

impl CfgNode {
    fn format_dot(&self, index: usize) -> String {
        let (label, shape) = match self {
            CfgNode::Function(func) => (func.clone(), "Mdiamond"),
            CfgNode::Precondition(pre) => (format!("Pre: {}", pre), "ellipse"), // Use "ellipse" shape
            CfgNode::Postcondition(post) => (format!("Post: {}", post), "ellipse"), // Use "ellipse" shape
            CfgNode::Invariant(inv) => (format!("Inv: {}", inv), "ellipse"), // Use "ellipse" shape
            CfgNode::Statement(stmt) => (stmt.clone(), "box"),
            CfgNode::Condition(cond) => (cond.clone(), "diamond"),
            CfgNode::MergePoint => (String::from("Merge"), "circle"), // Use "circle" shape
            CfgNode::Return(ret) => (format!("return: {}", ret), "ellipse"), // Format for return nodes
        };

        format!("{} [label=\"{}\", shape={}]", index, self.escape_quotes_for_dot(&label), shape)
    }

    fn escape_quotes_for_dot(&self, input: &str) -> String {
        input.replace("\"", "\\\"")
    }
}


struct CfgBuilder<'a> {
    graph: DiGraph<CfgNode, &'a str>,
    current_node: Option<NodeIndex>,
    next_edge_label: Option<&'a str>, // New field to store the label for the next edge
}


impl<'a> CfgBuilder<'a> {
    fn new() -> Self {
        CfgBuilder {
            graph: DiGraph::new(),
            current_node: None,
            next_edge_label: None, // Initialize the new field
        }
    }

    fn add_node(&mut self, node: CfgNode) -> NodeIndex {
        let index = self.graph.add_node(node);
        if let Some(current) = self.current_node {
            // Use the label for the next edge if available
            let label = self.next_edge_label.unwrap_or("");
            self.graph.add_edge(current, index, label);
            // Reset the edge label
            self.next_edge_label = None;
        }
        self.current_node = Some(index);
        index
    }

    fn to_dot(&self) -> String {
        let mut dot_string = String::new();
        dot_string.push_str("digraph G {\n");
    
        // Add nodes
        for node in self.graph.node_indices() {
            let cfg_node = &self.graph[node];
            dot_string.push_str(&cfg_node.format_dot(node.index()));
            dot_string.push('\n');
        }
    
        // Add edges with labels
        for edge in self.graph.edge_references() {
            let source = edge.source().index();
            let target = edge.target().index();
            let label = edge.weight();
            dot_string.push_str(&format!("{} -> {} [label=\"{}\"];\n", source, target, label));
        }
    
        dot_string.push_str("}\n");
        dot_string
    }

    // Helper method to format condition expressions
    fn format_condition(&self, expr: &Box<Expr>) -> String {
        quote!(#expr).to_string()
    }

    fn format_macro_args(&self, tokens: &proc_macro2::TokenStream) -> String {
        // Convert the entire token stream to a string
        let tokens_str = tokens.to_string();
        
        // Trim leading and trailing whitespace and quotation marks
        let trimmed_str = tokens_str.trim_start_matches("!(").trim_end_matches(')').trim_matches(|c| c == '"' || c == '\'').to_string();
        
        trimmed_str
    }

    fn add_edge_with_label(&mut self, from: NodeIndex, to: NodeIndex, label: &'a str) {
        self.graph.add_edge(from, to, label);
    }

    fn add_node_without_edge(&mut self, node: CfgNode) -> NodeIndex {
        let index = self.graph.add_node(node);
        self.current_node = Some(index);
        index
    }

    fn post_process(&mut self) {
        let mut to_remove = Vec::new();
        let mut edges_to_add = Vec::new();

        // Identify merge nodes and prepare edge adjustments
        for node in self.graph.node_indices() {
            if let CfgNode::MergePoint = &self.graph[node] {
                to_remove.push(node);

                // Collect outgoing edges of the merge node
                let outgoing_edges: Vec<_> = self.graph.edges(node).map(|e| (e.target(), *e.weight())).collect();

                // Redirect incoming edges of the merge node to its outgoing edges
                for edge in self.graph.edges_directed(node, petgraph::Direction::Incoming) {
                    let source = edge.source();
                    for (target, label) in &outgoing_edges {
                        edges_to_add.push((source, *target, *label));
                    }
                }
            }
        }

        // Add new edges
        for (source, target, label) in edges_to_add {
            self.graph.add_edge(source, target, label);
        }

        // Remove merge nodes
        for node in to_remove {
            self.graph.remove_node(node);
        }
    }
}

impl<'a, 'ast> Visit<'ast> for CfgBuilder<'a> {
    fn visit_file(&mut self, i: &'ast SynFile) {
        visit::visit_file(self, i);
    }

    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        let func_name = i.sig.ident.to_string();
        let func_node = self.add_node(CfgNode::Function(func_name));
        self.current_node = Some(func_node);

        for stmt in &i.block.stmts {
            match stmt {
                Stmt::Semi(expr, _) => {
                    if let Expr::Macro(expr_macro) = expr {
                        if let Some(macro_ident) = expr_macro.mac.path.get_ident() {
                            let macro_name = macro_ident.to_string();
                            let macro_args = self.format_macro_args(&expr_macro.mac.tokens);
                            let node = match macro_name.as_str() {
                                "pre" => CfgNode::Precondition(macro_args),
                                "post" => CfgNode::Postcondition(macro_args),
                                "invariant" => CfgNode::Invariant(macro_args),
                                _ => CfgNode::Statement(macro_args),
                            };
                            self.add_node(node);
                        } else {
                            self.visit_expr(expr);
                        }
                    } else {
                        self.visit_expr(expr);
                    }
                },
                _ => self.visit_stmt(stmt),
            }
        }

        self.current_node = None;
    }

    fn visit_stmt(&mut self, i: &'ast Stmt) {
        match i {
            Stmt::Local(local) => {
                // Handle local variable declarations
                let local_str = format!("{}", quote!(#local));
                self.add_node(CfgNode::Statement(local_str));
            }
            Stmt::Expr(expr) | Stmt::Semi(expr, _) => self.visit_expr(expr),
            _ => visit::visit_stmt(self, i),
        }
    }

    fn visit_expr(&mut self, i: &'ast Expr) {
        match i {
            Expr::If(expr_if) => {
                let cond_str = self.format_condition(&expr_if.cond);
                let cond_node = self.add_node(CfgNode::Condition(format!("if: {}", cond_str)));

                // Processing the true branch
                self.next_edge_label = Some("true");
                self.current_node = Some(cond_node.clone());
                self.visit_block(&expr_if.then_branch);
                let true_branch_end = self.current_node;
    
                // Processing the else branch, if it exists
                let false_branch_end = if let Some((_, else_branch)) = &expr_if.else_branch {
                    self.current_node = Some(cond_node.clone());
                    self.next_edge_label = Some("false");
                    match &**else_branch {
                        Expr::If(elseif) => {
                            self.visit_expr_if(elseif);
                            self.current_node
                        },
                        Expr::Block(block) => {
                            self.visit_block(&block.block);
                            self.current_node
                        },
                        _ => {
                            self.visit_expr(else_branch);
                            self.current_node
                        },
                    }
                } else {
                    None
                };
    
                // Create a merge point node (you might need a new variant in CfgNode for this)
                let merge_node = self.add_node_without_edge(CfgNode::MergePoint);

                // Connect the ends of both branches to the merge point
                if let Some(true_end) = true_branch_end {
                    self.add_edge_with_label(true_end, merge_node, "");
                }
                if let Some(false_end) = false_branch_end {
                    self.add_edge_with_label(false_end, merge_node, "");
                } else {
                    // If there is no else branch, connect the condition node to the merge point
                    self.add_edge_with_label(cond_node, merge_node, "");
                }

                // Continue from the merge point after if-else
                self.current_node = Some(merge_node);
            },     
            Expr::While(expr_while) => {
                // Extract and format only the condition of the while expression
                let cond_str = self.format_condition(&expr_while.cond);
                let cond_node = self.add_node(CfgNode::Condition(format!("while: {}", cond_str)));
                self.current_node = Some(cond_node);
                self.visit_block(&expr_while.body);
            },
            Expr::Return(expr_return) => {
                let return_expr = expr_return.expr.as_ref().map(|expr| quote!(#expr).to_string()).unwrap_or_else(|| String::from(""));
                let return_node = self.add_node(CfgNode::Return(return_expr));
                self.current_node = Some(return_node);
            },
            // ... [handle other expressions] ...
            _ => {
                let expr_str = quote!(#i).to_string();
                self.add_node(CfgNode::Statement(expr_str));
            },
        }
    }

    fn visit_block(&mut self, i: &'ast Block) {
        for stmt in &i.stmts {
            self.visit_stmt(stmt);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <input_filename>", args[0]);
        std::process::exit(1);
    }
    let filename = &args[1];

    let content = fs::read_to_string(filename).expect("Could not read file");
    let ast = syn::parse_file(&content).expect("Unable to parse file");

    let mut builder = CfgBuilder::new();
    builder.visit_file(&ast);

    // Post processing to remove merge nodes.

    builder.post_process();
    let dot_format = builder.to_dot();

    let mut dot_filename = filename.clone();
    if dot_filename.ends_with(".rs") {
        dot_filename.truncate(dot_filename.len() - 3);
    }

    let mut full_path = std::env::current_dir().expect("Unable to get current directory");
    full_path.push("graphs");
    fs::create_dir_all(&full_path).expect("Unable to create graphs directory");
    full_path.push(format!("{}.dot", dot_filename));

    let mut dot_file = File::create(&full_path).expect("Unable to create DOT file");
    dot_file.write_all(dot_format.as_bytes()).expect("Unable to write to DOT file");

    println!("DOT file saved as {:?}", full_path);
}