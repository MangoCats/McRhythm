use strsim::normalized_levenshtein;

fn main() {
    let mb_norm = "carlos santana";
    let src_norm = "santana";
    
    println!("Normalized names:");
    println!("  MB: '{}'", mb_norm);
    println!("  Source: '{}'", src_norm);
    println!();
    
    // Token-based similarity (Jaccard)
    let mb_tokens: Vec<&str> = mb_norm.split_whitespace().collect();
    let src_tokens: Vec<&str> = src_norm.split_whitespace().collect();
    
    println!("Tokens:");
    println!("  MB: {:?}", mb_tokens);
    println!("  Source: {:?}", src_tokens);
    println!();
    
    let intersection = mb_tokens.iter().filter(|t| src_tokens.contains(t)).count();
    let union_set: Vec<&str> = {
        let mut all = mb_tokens.clone();
        for t in &src_tokens {
            if !all.contains(t) {
                all.push(t);
            }
        }
        all
    };
    
    let jaccard_sim = intersection as f64 / union_set.len() as f64;
    println!("Jaccard similarity: {:.3} ({} / {})", jaccard_sim, intersection, union_set.len());
    
    // Character-based similarity (normalized Levenshtein)
    let levenshtein_sim = normalized_levenshtein(mb_norm, src_norm);
    println!("Levenshtein similarity: {:.3}", levenshtein_sim);
    println!();
    
    // Base similarity
    let base_similarity = jaccard_sim.max(levenshtein_sim);
    println!("Base similarity (max): {:.3}", base_similarity);
    println!();
    
    // Substring bonus
    let has_substring = mb_norm.contains(src_norm) || src_norm.contains(mb_norm);
    println!("Substring match: {}", has_substring);
    let prefix_bonus = if has_substring { 0.20 } else { 0.0 };
    println!("Substring bonus: {:.2}", prefix_bonus);
    println!();
    
    // Token subset bonus
    let is_subset = mb_tokens.iter().all(|t| src_tokens.contains(t)) || 
                    src_tokens.iter().all(|t| mb_tokens.contains(t));
    println!("Token subset: {}", is_subset);
    let token_subset_bonus = if is_subset { 0.15 } else { 0.0 };
    println!("Token subset bonus: {:.2}", token_subset_bonus);
    println!();
    
    let bonus = prefix_bonus.max(token_subset_bonus);
    println!("Max bonus applied: {:.2}", bonus);
    println!();
    
    let final_sim = (base_similarity + bonus).min(1.0);
    println!("Final similarity: {:.3}", final_sim);
}
