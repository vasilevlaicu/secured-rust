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


struct CfgBuilder {
    graph: DiGraph<CfgNode, String>,
    current_node: Option<NodeIndex>,
    next_edge_label: Option<String>,
}


impl CfgBuilder {
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
            let label = self.next_edge_label.clone().unwrap_or_else(|| "".to_string());
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

    fn add_edge_with_label(&mut self, from: NodeIndex, to: NodeIndex, label: String) {
        self.graph.add_edge(from, to, label);
    }
    

    fn add_node_without_edge(&mut self, node: CfgNode) -> NodeIndex {
        let index = self.graph.add_node(node);
        self.current_node = Some(index);
        index
    }

    fn post_process(&mut self) {
        let mut merge_nodes_to_process: Vec<NodeIndex> = self.graph.node_indices()
            .filter(|&n| matches!(self.graph[n], CfgNode::MergePoint))
            .collect();

        while let Some(merge_node) = merge_nodes_to_process.pop() {
            // Check if the merge node has edges (i.e., is still part of the graph)
            if self.graph.edges(merge_node).count() == 0 {
                continue;
            }

            // Find outgoing edges of the merge node
            let edges: Vec<_> = self.graph.edges(merge_node).collect();

            if edges.len() == 1 {
                let target = edges[0].target();
                if matches!(self.graph[target], CfgNode::MergePoint) {
                    // If the target is another merge node, merge them
                    self.merge_merge_nodes(merge_node, target);
                    merge_nodes_to_process.push(target);
                } else {
                    // If the target is not a merge node, redirect incoming edges and remove the merge node
                    self.redirect_edges_and_remove(merge_node, target);
                }
            }
        }
    }

    fn merge_merge_nodes(&mut self, source: NodeIndex, target: NodeIndex) {
        let incoming_edges: Vec<_> = self.graph.edges_directed(source, petgraph::Direction::Incoming)
            .map(|e| (e.source(), e.weight().clone()))
            .collect();
    
        for (source_of_edge, weight) in incoming_edges {
            self.graph.add_edge(source_of_edge, target, weight);
        }
    
        self.graph.remove_node(source);
    }
    
    fn redirect_edges_and_remove(&mut self, source: NodeIndex, new_target: NodeIndex) {
        let incoming_edges: Vec<_> = self.graph.edges_directed(source, petgraph::Direction::Incoming)
            .map(|e| (e.source(), e.weight().clone()))
            .collect();
    
        for (source_of_edge, weight) in incoming_edges {
            self.graph.add_edge(source_of_edge, new_target, weight);
        }
    
        self.graph.remove_node(source);
    }
    
    fn handle_if_statement(&mut self, expr_if: &syn::ExprIf) {
        let cond_str = self.format_condition(&expr_if.cond);
        let cond_label = if self.next_edge_label == Some("false".to_string()) {
            format!("else if: {}", cond_str)
        } else {
            format!("if: {}", cond_str)
        };
        let cond_node = self.add_node(CfgNode::Condition(cond_label));
    
        // Processing the true branch
        self.next_edge_label = Some("true".to_string());
        self.current_node = Some(cond_node.clone());
        self.visit_block(&expr_if.then_branch);
        let true_branch_end = self.current_node;
    
        // Create a merge point node
        let merge_node = self.add_node_without_edge(CfgNode::MergePoint);
    
        // Connect the true branch end to the merge point
        if let Some(true_end) = true_branch_end {
            self.add_edge_with_label(true_end, merge_node, "".to_string());
        }
    
        // Handling the else branch, if it exists
        let false_branch_end = if let Some((_, else_branch)) = &expr_if.else_branch {
            self.current_node = Some(cond_node.clone());
            self.next_edge_label = Some("false".to_string());
            match &**else_branch {
                Expr::If(elseif) => {
                    // Recursively handle else if
                    self.handle_if_statement(elseif);
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
    
        // Connect the ends of the else branch to the merge point
        if let Some(false_end) = false_branch_end {
            self.add_edge_with_label(false_end, merge_node, "".to_string());
        } else {
            // If there is no else branch, connect the condition node to the merge point
            self.add_edge_with_label(cond_node, merge_node, "".to_string());
        }
    
        // Continue from the merge point after if-else
        self.current_node = Some(merge_node);
    }
}

impl Visit<'_> for CfgBuilder {
    fn visit_file(&mut self, i: &SynFile) {
        visit::visit_file(self, i);
    }

    fn visit_item_fn(&mut self, i: &ItemFn) {
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

    fn visit_stmt(&mut self, i: &Stmt) {
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

    fn visit_expr(&mut self, i: &Expr) {
        match i {
            Expr::If(expr_if) => {
                self.handle_if_statement(expr_if);
            },
            Expr::While(expr_while) => {
                // Check if the last node was an invariant
                let invariant_node = self.current_node
                    .filter(|&current| matches!(self.graph[current], CfgNode::Invariant(_)));
    
                // Create a node for the while condition
                let cond_str = self.format_condition(&expr_while.cond);
                let cond_node = self.add_node(CfgNode::Condition(format!("while: {}", cond_str)));
    
                // Link the invariant to the condition node only if it exists and hasn't been linked yet
                if let Some(inv_node) = invariant_node {
                    if self.graph.find_edge(inv_node, cond_node).is_none() {
                        self.add_edge_with_label(inv_node, cond_node, "".to_string());
                    }
                }
    
                // Process the loop body
                self.current_node = Some(cond_node);
                self.next_edge_label = Some("true".to_string());
                self.visit_block(&expr_while.body);
    
                // Link back to the invariant or condition node after the loop body
                let loop_back_node = invariant_node.unwrap_or(cond_node);
                self.add_edge_with_label(self.current_node.unwrap(), loop_back_node, "back to loop".to_string());
    
                // Create a merge node for the false branch of the condition
                let merge_node = self.add_node_without_edge(CfgNode::MergePoint);
                self.add_edge_with_label(cond_node, merge_node, "false".to_string());
    
                // Continue from the merge point
                self.current_node = Some(merge_node);
            },
            Expr::Return(expr_return) => {
                let return_expr = expr_return.expr.as_ref().map(|expr| quote!(#expr).to_string()).unwrap_or_else(|| String::from(""));
                let return_node = self.add_node(CfgNode::Return(return_expr));
                self.current_node = Some(return_node);
            },
            // ... [handle other expressions] ...
            _ => {
                // Handle invariant macro here
                if let Expr::Macro(expr_macro) = i {
                    if let Some(macro_ident) = expr_macro.mac.path.get_ident() {
                        if macro_ident == "invariant" {
                            // Handle invariant
                            let invariant_str = self.format_macro_args(&expr_macro.mac.tokens);
                            self.add_node(CfgNode::Invariant(invariant_str));
                            return;
                        }
                    }
                }
    
                let expr_str = quote!(#i).to_string();
                self.add_node(CfgNode::Statement(expr_str));
            },
        }
    }

    fn visit_block(&mut self, i: &Block) {
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