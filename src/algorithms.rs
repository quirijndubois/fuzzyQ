use crate::structs::Suggestion;

pub fn normalize_embeddings(embeddings: &mut [Vec<f32>]) {
    for emb in embeddings.iter_mut() {
        let norm = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in emb.iter_mut() {
                *v /= norm;
            }
        }
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    // we assume normalized vector to apply function simplification (not dividing by norms)
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    dot
}

pub fn fuzzy_match(query: &str, candidate: &str) -> Option<Suggestion> {
    let q = query.to_lowercase();
    let c = candidate.to_lowercase();

    let mut score: usize = 0;
    let mut match_indices: Vec<usize> = Vec::new();

    // 1. Exact match
    if q == c {
        score = 1000;
        match_indices = (0..q.len()).collect();
        return Some(Suggestion {
            text: candidate.to_string(),
            match_indices,
            score,
        });
    }

    // 2. Substring match
    if let Some(pos) = c.find(&q) {
        score += 200;
        score += q.len() * 10;
        score += 100usize.saturating_sub(pos); // earlier is better
        match_indices = (pos..pos + q.len()).collect();
    }

    // 3. Prefix bonus
    if c.starts_with(&q) {
        score += 150;
    }

    // 4. Subsequence match (always attempt)
    let mut last = 0;
    let mut gaps = 0;

    for qc in q.chars() {
        if let Some(pos) = c[last..].find(qc) {
            let real = last + pos;
            if let Some(prev) = match_indices.last() {
                gaps += real.saturating_sub(*prev + 1);
            }
            match_indices.push(real);
            last = real + 1;
        }
    }

    let matched = match_indices.len();
    if matched > 0 {
        score += matched * 10;
        score += 50usize.saturating_sub(gaps);
    }

    // 5. Edit distance bonus (handles "heyp" -> "hey")
    let dist = levenshtein(&q, &c);
    if dist <= 2 {
        score += (3 - dist) * 30;
    }

    // 6. clamp score to 0 - 1000
    if score > 1000 {
        score = 1000;
    }

    Some(Suggestion {
        text: candidate.to_string(),
        match_indices,
        score,
    })
}

fn levenshtein(a: &str, b: &str) -> usize {
    let mut costs: Vec<usize> = (0..=b.len()).collect();

    for (i, ca) in a.chars().enumerate() {
        let mut last = i;
        costs[0] = i + 1;

        for (j, cb) in b.chars().enumerate() {
            let new = if ca == cb {
                last
            } else {
                1 + last.min(costs[j]).min(costs[j + 1])
            };
            last = costs[j + 1];
            costs[j + 1] = new;
        }
    }

    costs[b.len()]
}
