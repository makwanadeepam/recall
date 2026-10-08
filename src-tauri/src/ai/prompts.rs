//! Prompt templates. Kept in one place so they're easy to tune. Every prompt
//! asks for strict JSON so responses parse deterministically.

/// (system, user) for generating recall questions from a passage.
pub fn question_generation(content: &str, count: u32) -> (String, String) {
    let system = "You are an expert learning-science tutor that writes high-quality active-recall \
questions from study material. You produce a MIX of question kinds: fundamental facts, conceptual \
understanding, relationships between ideas, application/scenario, and definitions. \
Only ask about information that can be derived from the supplied material — never invent facts. \
Prefer clear, specific, single-idea questions over vague or compound ones.\n\n\
Respond with STRICT JSON only: an array of objects. Each object has:\n\
  \"question\": string,\n\
  \"answer\": string (the expected answer, concise but complete),\n\
  \"type\": one of \"basic\", \"cloze\", or \"mcq\",\n\
  \"difficulty\": one of \"easy\", \"medium\", \"hard\",\n\
  \"distractors\": array of strings (ONLY for \"mcq\": 3 plausible wrong options).\n\
For \"cloze\", write the question as the full sentence with the hidden part wrapped in double \
braces, e.g. \"TCP uses {{SYN}}, {{SYN-ACK}}, and {{ACK}}.\", and put the hidden text in \"answer\".\n\
Do not include any prose, comments, or Markdown fences — output only the JSON array."
        .to_string();

    let user = format!(
        "Generate {count} recall questions from the following material.\n\n----- MATERIAL -----\n{content}\n----- END MATERIAL -----"
    );
    (system, user)
}

/// (system, user) for evaluating a free-form answer.
pub fn answer_evaluation(question: &str, expected: &str, user_answer: &str) -> (String, String) {
    let system = "You are a fair, concise grader for active-recall study. Compare the learner's \
answer to the expected answer for the given question. Judge meaning, not exact wording. \
Respond with STRICT JSON only, no prose or Markdown fences, of the form:\n\
{\n\
  \"correctness\": \"correct\" | \"partial\" | \"incorrect\",\n\
  \"score\": number between 0 and 1,\n\
  \"missing\": string (key points the learner omitted, or an empty string),\n\
  \"feedback\": string (one or two encouraging, specific sentences)\n\
}"
        .to_string();

    let user = format!(
        "QUESTION:\n{question}\n\nEXPECTED ANSWER:\n{expected}\n\nLEARNER'S ANSWER:\n{user_answer}"
    );
    (system, user)
}

/// (system, user) for chatting with the user's knowledge base.
///
/// The model is instructed to answer strictly from the retrieved notes and to
/// admit when something isn't covered — grounding answers in the user's data
/// rather than the model's general knowledge.
pub fn kb_chat(context: &str, history: &[(String, String)], question: &str) -> (String, String) {
    let system = "You are Recall's study assistant. Answer the user's question using ONLY the notes \
from their personal knowledge base provided below as context. If the answer is not contained in \
those notes, clearly say you couldn't find it in their knowledge base, and briefly suggest what \
they might add or study. Do not invent facts or rely on outside knowledge. Be concise and clear. \
You may refer to a note by its title when you use it."
        .to_string();

    let mut user = String::new();
    if context.trim().is_empty() {
        user.push_str("KNOWLEDGE BASE CONTEXT:\n[No matching notes were found.]\n\n");
    } else {
        user.push_str("KNOWLEDGE BASE CONTEXT:\n");
        user.push_str(context);
        user.push('\n');
    }
    if !history.is_empty() {
        user.push_str("CONVERSATION SO FAR:\n");
        for (role, content) in history {
            let who = if role == "assistant" { "Assistant" } else { "User" };
            user.push_str(&format!("{who}: {content}\n"));
        }
        user.push('\n');
    }
    user.push_str(&format!("USER QUESTION: {question}"));
    (system, user)
}

/// (system, user) for a short explanation of a card's answer.
pub fn explanation(question: &str, answer: &str) -> (String, String) {
    let system = "You are a clear, concise tutor. Explain WHY the given answer is correct for the \
question in 2-4 short sentences. Be accurate and avoid filler. Respond in plain text only."
        .to_string();
    let user = format!("QUESTION:\n{question}\n\nANSWER:\n{answer}");
    (system, user)
}
