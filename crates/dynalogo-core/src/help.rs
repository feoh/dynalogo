use crate::generated_help::{GeneratedHelpTopic, HELP_TOPICS};

pub fn overview() -> String {
    let mut categories = HELP_TOPICS
        .iter()
        .map(|topic| topic.category)
        .collect::<Vec<_>>();
    categories.sort_unstable();
    categories.dedup();

    let mut output = String::from("DynaLOGO help topics\n\nCategories:\n");
    for category in categories {
        let count = HELP_TOPICS
            .iter()
            .filter(|topic| topic.category == category)
            .count();
        output.push_str(&format!("  {category} ({count})\n"));
    }
    output.push_str("\nTry: HELP \"fd, HELP \"lists, HELP \"window-input, or APROPOS \"turtle");
    output
}

pub fn topic(query: &str) -> Option<&'static GeneratedHelpTopic> {
    let query = normalize_query(query);
    HELP_TOPICS
        .iter()
        .find(|topic| topic_matches(topic, &query))
}

pub fn search(query: &str) -> Vec<&'static GeneratedHelpTopic> {
    let query = normalize_query(query);
    if query.is_empty() {
        return HELP_TOPICS.iter().collect();
    }

    HELP_TOPICS
        .iter()
        .filter(|topic| topic_search_text(topic).contains(&query))
        .collect()
}

pub fn suggestions(query: &str) -> Vec<&'static GeneratedHelpTopic> {
    let query = normalize_query(query);
    if query.is_empty() {
        return Vec::new();
    }

    let mut suggestions = search(&query);
    if suggestions.is_empty() {
        suggestions = HELP_TOPICS
            .iter()
            .filter(|topic| {
                topic_lookup_names(topic)
                    .iter()
                    .any(|name| edit_distance(name, &query) <= 2)
            })
            .collect();
    }
    suggestions.truncate(5);
    suggestions
}

pub fn format_topic(topic: &GeneratedHelpTopic) -> String {
    let mut output = String::new();
    output.push_str(topic.title);
    output.push('\n');
    if let Some(signature) = topic.signature {
        output.push_str(&format!("Signature: {signature}\n"));
    }
    if !topic.aliases.is_empty() {
        output.push_str(&format!("Aliases: {}\n", topic.aliases.join(", ")));
    }
    output.push('\n');
    output.push_str(topic.summary);
    output.push_str("\n\n");
    output.push_str(topic.body);
    if !topic.see_also.is_empty() {
        output.push_str("\n\nSee also: ");
        output.push_str(&topic.see_also.join(", "));
    }
    output
}

pub fn format_search(query: &str, topics: &[&GeneratedHelpTopic]) -> String {
    if topics.is_empty() {
        return format!("No help topics match {query:?}.");
    }

    let mut output = format!("Help topics matching {query:?}:\n");
    for topic in topics {
        output.push_str(&format!("  {} — {}\n", topic.id, topic.summary));
    }
    output.trim_end().to_string()
}

pub fn format_unknown(query: &str) -> String {
    let suggestions = suggestions(query);
    if suggestions.is_empty() {
        format!("No help topic named {query:?}. Try APROPOS \"keyword.")
    } else {
        let names = suggestions
            .iter()
            .map(|topic| topic.id)
            .collect::<Vec<_>>()
            .join(", ");
        format!("No help topic named {query:?}. Did you mean: {names}?")
    }
}

fn topic_matches(topic: &GeneratedHelpTopic, query: &str) -> bool {
    topic_lookup_names(topic).iter().any(|name| name == query)
}

fn topic_lookup_names(topic: &GeneratedHelpTopic) -> Vec<String> {
    std::iter::once(topic.id)
        .chain(topic.names.iter().copied())
        .chain(topic.aliases.iter().copied())
        .map(normalize_query)
        .collect()
}

fn topic_search_text(topic: &GeneratedHelpTopic) -> String {
    std::iter::once(topic.id)
        .chain(std::iter::once(topic.title))
        .chain(topic.names.iter().copied())
        .chain(topic.aliases.iter().copied())
        .chain(std::iter::once(topic.summary))
        .chain(std::iter::once(topic.category))
        .chain(topic.tags.iter().copied())
        .map(normalize_query)
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_query(query: &str) -> String {
    query.trim().trim_start_matches('"').to_ascii_lowercase()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let mut previous = (0..=b.len()).collect::<Vec<_>>();
    let mut current = vec![0; b.len() + 1];

    for (i, a_byte) in a.bytes().enumerate() {
        current[0] = i + 1;
        for (j, b_byte) in b.bytes().enumerate() {
            let substitution = usize::from(a_byte != b_byte);
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_lookup_accepts_ids_names_and_aliases() {
        assert_eq!(topic("fd").map(|topic| topic.id), Some("fd"));
        assert_eq!(topic("FORWARD").map(|topic| topic.id), Some("fd"));
        assert_eq!(topic("\"FD").map(|topic| topic.id), Some("fd"));
    }

    #[test]
    fn search_finds_tags_and_summaries() {
        let results = search("window");
        assert!(results.iter().any(|topic| topic.id == "window-input"));
    }

    #[test]
    fn grouped_primitive_topics_are_lookup_targets() {
        assert_eq!(topic("sum").map(|topic| topic.id), Some("arithmetic"));
        assert_eq!(topic("print").map(|topic| topic.id), Some("console-files"));
        assert_eq!(topic("tell").map(|topic| topic.id), Some("dynaturtles"));
    }

    #[test]
    fn suggestions_include_close_topic_ids() {
        let results = suggestions("fdd");
        assert!(results.iter().any(|topic| topic.id == "fd"));
    }
}
