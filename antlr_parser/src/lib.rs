use std::char;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

enum State {
    ReadNextChar,
    InBrackets { depth: u8 },
    CheckForSubrulesNormal,
    CheckForSubrulesRegex,
    InAction,
    InRegex,
    FoundDot,
    FoundTwoDots,
    FoundNot { depth: u8 },
}

pub struct AntlrParser {
    nonterminals: Vec<(String, String)>, //First is the original name, second the uppercase name
    pub rules: Vec<(String, String)>,
}

impl AntlrParser {
    pub fn new() -> AntlrParser {
        AntlrParser {
            nonterminals: vec![],
            rules: vec![],
        }
    }

    //Adds a new Tuple to nonterminals. The first string contains the original name the second string contains its new name (Uppercase for grammartec syntax).
    pub fn add_nonterm(&mut self, string: String) {
        let mut new_nonterm = string.to_uppercase();
        loop {
            if !self.is_nonterm_name(&new_nonterm) {
                break;
            }
            new_nonterm.push('1');
        }
        self.nonterminals.push((string, new_nonterm));
    }
    pub fn is_nonterm_name(&self, string: &str) -> bool {
        for nt in &self.nonterminals {
            if nt.1 == string {
                return true;
            }
        }
        return false;
    }

    //This function parses an antlr grammar
    pub fn parse_antlr_grammar(&mut self, file: &str) {
        let f = File::open(file).expect("file not found");
        let file = BufReader::new(&f);
        let mut lines = file.lines();
        let mut pre_body = String::new();
        let mut body = String::new();

        loop {
            //find start line;
            if lines
                .next()
                .expect("RAND_842445070")
                .expect("RAND_842445070")
                .starts_with("grammar")
            {
                break;
            }
        }

        //Combine the rest to one string and remove comments
        for line in lines {
            pre_body.push_str(&(self.remove_single_line_comment(line.expect("RAND_3089939874"))));
            pre_body.push('\n');
        }

        //Replace escaped unicode
        pre_body = self.replace_unicode(pre_body);
        pre_body = self.basic_editing(&pre_body);

        //Remove unneeded stuff and mark subrules
        for rule in pre_body.split(';') {
            let rule_without_spaces = rule.replace(" ", "");
            if rule_without_spaces.ends_with("->skip") {
                continue;
            } //We don't need the ignore conditions
            let edited_rule = self.remove_fragment(rule.trim().to_string());
            body.push_str(&(self.tokenize_subrules(&edited_rule) + ";"));
        }

        //Find all nonterms
        for rule in body.split(';') {
            if rule.contains(":") {
                let rule_name = rule.split(':').next().expect("RAND_913712400").trim();
                if rule_name != "" {
                    self.add_nonterm(rule_name.to_string());
                }
            }
        }

        //Iterate through all Rules:
        for rule in body.split(';') {
            if rule.contains(":") {
                //Progress the rule definition
                let mut name_and_definition = rule.splitn(2, ':'); //splitn because there could be another ':' in a rule definition
                let name = name_and_definition.next().expect("RAND_1336718586").trim();
                let definition = name_and_definition.next().expect("RAND_2265765111").trim();
                let mut definitions = self.parse_definition(&definition, name);
                for def in definitions.iter_mut() {
                    self.print_string(name, def);
                }
            }
        }
    }

    //This function removes fragment form the beginning of a string
    fn remove_fragment(&self, mut string: String) -> String {
        if string.starts_with("fragment ") {
            return string.split_off(9);
        }
        return string;
    }

    //This function removes single line comments from a line
    fn remove_single_line_comment(&self, string: String) -> String {
        let mut current = String::new();
        let mut last_char = 'a';
        let mut in_quotes = false; //Variable to "count" "'"
        for character in string.chars() {
            match character {
                '\'' if last_char != '\\' => {
                    current.push('\'');
                    in_quotes ^= true
                }
                '/' if !in_quotes && (last_char == '/') => {
                    return current;
                }
                '/' if !in_quotes && (last_char != '/') => {}
                _ if last_char == '/' => {
                    current.push('/');
                    current.push(character);
                }
                _ => current.push(character),
            }
            last_char = character;
        }
        return current;
    }

    //check for *, +, ?, *?, +? after words without brackets around them
    //Works on a single line or rule
    fn tokenize_subrules(&self, string: &str) -> String {
        let mut current = String::new();
        let mut current_word = String::new();
        let mut last_char = 'a';
        let mut in_quotes = false; //Variable to "count" "'"
        for character in string.chars() {
            match character {
                '\'' if last_char != '\\' => {
                    current_word.push('\'');
                    in_quotes ^= true
                }
                ' ' | '\n' | '\r' | '\t' | ';' if !in_quotes => {
                    if (!current_word.ends_with(")*")
                        && !current_word.ends_with("]*")
                        && !current_word.ends_with(")+")
                        && !current_word.ends_with("]+")
                        && !current_word.ends_with(")?")
                        && !current_word.ends_with("]?")
                        && !current_word.ends_with("*?")
                        && !current_word.ends_with("+?"))
                        && (current_word.ends_with("*")
                            || current_word.ends_with("+")
                            || current_word.ends_with("?"))
                    {
                        let subrule = current_word
                            .pop()
                            .expect("String did not contain any characters");
                        current.push('(');
                        current.push_str(&current_word);
                        current.push(')');
                        current.push(subrule);
                    } else if (!current_word.ends_with(")*?")
                        && !current_word.ends_with("]*?")
                        && !current_word.ends_with(")+?")
                        && !current_word.ends_with("]+?"))
                        && (current_word.ends_with("*?") || current_word.ends_with("+?"))
                    {
                        current_word.pop();
                        let subrule = current_word
                            .pop()
                            .expect("String did not contain any characters");
                        current.push('(');
                        current.push_str(&current_word);
                        current.push(')');
                        current.push(subrule);
                        current.push('?');
                    } else {
                        current.push_str(&current_word);
                    }
                    current.push(' ');
                    current_word = String::new();
                }
                _ => current_word.push(character),
            }
            last_char = character;
        }
        if (!current_word.ends_with(")*")
            && !current_word.ends_with("]*")
            && !current_word.ends_with(")+")
            && !current_word.ends_with("]+")
            && !current_word.ends_with(")?")
            && !current_word.ends_with("]?")
            && !current_word.ends_with("*?")
            && !current_word.ends_with("+?"))
            && (current_word.ends_with("*")
                || current_word.ends_with("+")
                || current_word.ends_with("?"))
        {
            let subrule = current_word
                .pop()
                .expect("String did not contain any characters");
            current.push('(');
            current.push_str(&current_word);
            current.push(')');
            current.push(subrule);
        } else if (!current_word.ends_with(")*?")
            && !current_word.ends_with("]*?")
            && !current_word.ends_with(")+?")
            && !current_word.ends_with("]+?"))
            && (current_word.ends_with("*?") || current_word.ends_with("+?"))
        {
            current_word.pop();
            let subrule = current_word
                .pop()
                .expect("String did not contain any characters");
            current.push('(');
            current.push_str(&current_word);
            current.push(')');
            current.push(subrule);
            current.push('?');
        } else {
            current.push_str(&current_word);
        }
        current.push(' ');
        current_word = String::new();
        if current_word.len() > 0 {
            current.push_str(&current_word);
        }
        return current;
    }

    //This function checks if a word is a nonterm
    fn is_nonterm(&self, string: &str) -> bool {
        for nt in &self.nonterminals {
            if nt.0 == string {
                return true;
            }
        }
        return false;
    }

    //First remove forms like "id=nterm" and form them into "nterm" and escape basic stuff
    fn basic_editing(&self, string: &str) -> String {
        let mut current = String::new();
        let mut current_word = String::new();
        let mut last_char = 'a';
        let mut in_quotes = false; //Variable to "count" "'"
        let mut in_backslash_escape = false; //Variable to "count" "\"
        let mut in_brackets = false; //Varuiable to "count" "["
        for character in string.chars() {
            match character {
                '\'' if !in_backslash_escape && !in_brackets => {
                    current_word.push('\'');
                    in_quotes ^= true
                }
                '\\' if in_quotes => {
                    in_backslash_escape ^= true;
                    if !in_backslash_escape {
                        current_word.push_str("&escaped_backslash");
                    }
                }
                '=' if !in_quotes => {
                    current_word = String::new();
                } //remove the part before the "="
                ' ' if in_quotes => {
                    current_word.push_str(" &space ");
                }
                'n' if in_quotes && (last_char == '\\') => {
                    in_backslash_escape ^= true;
                    current_word.push_str("&escaped_n");
                }
                'r' if in_quotes && (last_char == '\\') => {
                    in_backslash_escape ^= true;
                    current_word.push_str("&escaped_r");
                }
                //'?' if !in_quotes => {}, //remove ?
                '{' if in_quotes => {
                    current_word.push_str("&escaped_curly_bracked_open");
                }
                '}' if in_quotes => {
                    current_word.push_str("&escaped_curly_bracked_closed");
                }
                '[' if !in_quotes && !in_backslash_escape => {
                    current_word.push('[');
                    in_brackets ^= true
                }
                ']' if !in_quotes && !in_backslash_escape && in_brackets => {
                    current_word.push(']');
                    in_brackets ^= true
                }
                ';' if in_quotes => {
                    current_word.push_str("&escaped_semicolon");
                }
                ' ' => {
                    current_word.push(' ');
                    current.push_str(&current_word);
                    current_word = String::new();
                }
                _ if in_backslash_escape => {
                    current_word.push('\\');
                    current_word.push(character);
                    in_backslash_escape ^= true;
                }
                _ => current_word.push(character),
            }
            last_char = character;
        }
        if current_word.len() > 0 {
            current.push_str(&current_word);
        }
        return current;
    }

    //This function takes a String and returns a Vec<String>
    fn parse_definition(&mut self, string: &str, name: &str) -> Vec<String> {
        let mut definitions: Vec<String> = vec![];
        let mut praeposition: Vec<String> = vec![];
        let mut current = String::new();
        let mut last_char = 'a';
        let mut in_quotes = false; //Variable to "count" "'"
        let mut in_backslash_escape = false; //Variable to "count" "\"
        let mut state = State::ReadNextChar;
        for character in string.chars() {
            match state {
                State::ReadNextChar => match character {
                    '\'' if !in_backslash_escape => {
                        in_quotes ^= true;
                    }
                    '\'' if in_quotes && in_backslash_escape => {
                        current.push_str("&escaped_single_quote");
                        in_backslash_escape ^= true;
                    }
                    '\\' if in_quotes => {
                        in_backslash_escape ^= true;
                        if in_backslash_escape == false {
                            current.push_str("&escaped_backslash");
                        }
                    }
                    '|' if !in_quotes => {
                        if praeposition.len() == 0 {
                            definitions.push(current.clone());
                        }
                        for definition in praeposition {
                            definitions.push(definition + &current);
                        }
                        praeposition = vec![];
                        current = String::new();
                    }
                    '[' if !in_quotes => {
                        if current.len() > 0 {
                            praeposition =
                                self.combine_vectors(&praeposition, &vec![current.clone()]);
                            current = String::new();
                        }
                        state = State::InRegex;
                    }
                    '(' if !in_quotes => {
                        if current.len() > 0 {
                            praeposition =
                                self.combine_vectors(&praeposition, &vec![current.clone()]);
                            current = String::new();
                        }
                        state = State::InBrackets { depth: 1 };
                    }
                    '{' if !in_quotes => {
                        state = State::InAction;
                    }
                    '.' if !in_quotes => {
                        state = State::FoundDot;
                    }
                    '~' if !in_quotes => {
                        if current.len() > 0 {
                            praeposition =
                                self.combine_vectors(&praeposition, &vec![current.clone()]);
                            current = String::new();
                        }
                        state = State::FoundNot { depth: 0 };
                    }
                    _ if in_backslash_escape => {
                        current.push('\\');
                        current.push(character);
                        in_backslash_escape ^= true;
                    }
                    _ => current.push(character),
                },
                State::FoundDot => {
                    match character {
                        '.' => {
                            state = State::FoundTwoDots;
                        }
                        _ => {
                            if current.len() > 0 {
                                praeposition =
                                    self.combine_vectors(&praeposition, &vec![current.clone()]);
                                current = String::new();
                            }
                            //let definition_helper = self.all_chars();
                            let definition_helper = vec!["Not(\"\")".to_string()];
                            praeposition = self.combine_vectors(&praeposition, &definition_helper);
                            state = State::ReadNextChar;
                        }
                    }
                }
                State::FoundTwoDots => match character {
                    '\'' => {
                        if in_quotes {
                            let mut regex = current.clone() + "-";
                            regex.push(last_char);
                            let definition_helper = self.parse_regex(&regex);
                            praeposition = self.combine_vectors(&praeposition, &definition_helper);
                            current = String::new();
                            state = State::ReadNextChar;
                        }
                        in_quotes ^= true;
                    }
                    _ => {}
                },
                State::FoundNot { depth: x } => {
                    match character {
                        /*'\'' if last_char != '\\' => {in_quotes ^= true;current.push(character);},
						'(' if !in_quotes => {
							state = State::FoundNot{depth: x+1};
							if x > 0 {current.push(character);}
						},
						')' if !in_quotes => {
							//if x == 1 {
								//let mut definition_helper = self.all_chars();
								//for string in self.parse_definition(&current, name) {
								//	definition_helper.remove_item(&string);	
								//}
								//current = String::new();
								//praeposition = self.combine_vectors(&praeposition, &definition_helper);
								//state = State::ReadNextChar;
							//}
							//else {
							//	state = State::FoundNot{depth: x-1};
							//	current.push(')');
							//}
						},
						_ => current.push(character)*/
                        '\'' if last_char != '\\' => {
                            in_quotes ^= true;
                            current.push(character);
                        }
                        '(' if !in_quotes => {
                            state = State::FoundNot { depth: x + 1 };
                            current.push(character);
                        }
                        ')' if !in_quotes => {
                            if x == 1 {
                                let mut not_string = String::from(" &spaceNot(");
                                for string in self.parse_definition(&current, name) {
                                    not_string.push_str(&string);
                                }
                                not_string.push_str(")&space)");
                                praeposition =
                                    self.combine_vectors(&praeposition, &vec![not_string]);
                                current = String::new();
                                state = State::ReadNextChar;
                            } else {
                                state = State::FoundNot { depth: x - 1 };
                                current.push(')');
                            }
                        }
                        _ => current.push(character),
                    }
                }
                State::InAction => match character {
                    '\'' if last_char != '\\' => {
                        in_quotes ^= true;
                    }
                    '}' if !in_quotes => state = State::ReadNextChar,
                    _ => {}
                },
                State::InBrackets { depth: x } => match character {
                    '\'' if last_char != '\\' => {
                        in_quotes ^= true;
                        current.push(character);
                    }
                    '(' if !in_quotes => {
                        state = State::InBrackets { depth: x + 1 };
                        current.push(character);
                    }
                    ')' if !in_quotes => {
                        if x == 1 {
                            state = State::CheckForSubrulesNormal;
                        } else {
                            state = State::InBrackets { depth: x - 1 };
                            current.push(')');
                        }
                    }
                    _ => current.push(character),
                },
                State::CheckForSubrulesNormal => {
                    match character {
                        '*' => {
                            let definition_helper = self.parse_definition(&current, name);
                            let new_rule_name = self.apply_mul_subrule(definition_helper, name);
                            current = String::new();
                            if praeposition.len() == 0 {
                                praeposition.push(String::new());
                            }
                            for prae in praeposition.iter_mut() {
                                prae.push_str(&(" ".to_string() + &new_rule_name + " "));
                            }
                            state = State::ReadNextChar;
                        }
                        '?' => {
                            let mut definition_helper = self.parse_definition(&current, name);
                            definition_helper.push(String::new());
                            current = String::new();
                            praeposition = self.combine_vectors(&praeposition, &definition_helper);
                            state = State::ReadNextChar;
                        }
                        '+' => {
                            let definition_helper = self.parse_definition(&current, name);
                            let new_rule_name = self.apply_plus_subrule(definition_helper, name);
                            current = String::new();
                            if praeposition.len() == 0 {
                                praeposition.push(String::new());
                            }
                            for prae in praeposition.iter_mut() {
                                prae.push_str(&(" ".to_string() + &new_rule_name + " "));
                            }
                            state = State::ReadNextChar;
                        }
                        '(' => {
                            let definition_helper = self.parse_definition(&current, name);
                            current = String::new();
                            praeposition = self.combine_vectors(&praeposition, &definition_helper);
                            state = State::InBrackets { depth: 1 };
                        }
                        '[' => {
                            let definition_helper = self.parse_definition(&current, name);
                            current = String::new();
                            praeposition = self.combine_vectors(&praeposition, &definition_helper);
                            state = State::InRegex;
                        }
                        _ => {
                            let definition_helper = self.parse_definition(&current, name);
                            //println!("{}: length: {}", current, definition_helper.len());
                            current = character.to_string();
                            praeposition = self.combine_vectors(&praeposition, &definition_helper);
                            state = State::ReadNextChar;
                        }
                    }
                }
                State::CheckForSubrulesRegex => match character {
                    '*' => {
                        let definition_helper = self.parse_regex(&current);
                        let new_rule_name = self.apply_mul_subrule(definition_helper, name);
                        current = String::new();
                        if praeposition.len() == 0 {
                            praeposition.push(String::new());
                        }
                        for prae in praeposition.iter_mut() {
                            prae.push_str(&(" ".to_string() + &new_rule_name + " "));
                        }
                        state = State::ReadNextChar;
                    }
                    '?' => {
                        let mut definition_helper = self.parse_regex(&current);
                        definition_helper.push(String::new());
                        current = String::new();
                        praeposition = self.combine_vectors(&praeposition, &definition_helper);
                        state = State::ReadNextChar;
                    }
                    '+' => {
                        let definition_helper = self.parse_regex(&current);
                        let new_rule_name = self.apply_plus_subrule(definition_helper, name);
                        current = String::new();
                        if praeposition.len() == 0 {
                            praeposition.push(String::new());
                        }
                        for prae in praeposition.iter_mut() {
                            prae.push_str(&(" ".to_string() + &new_rule_name + " "));
                        }
                        state = State::ReadNextChar;
                    }
                    '(' => {
                        let definition_helper = self.parse_definition(&current, name);
                        current = String::new();
                        praeposition = self.combine_vectors(&praeposition, &definition_helper);
                        state = State::InBrackets { depth: 1 };
                    }
                    '[' => {
                        let definition_helper = self.parse_regex(&current);
                        current = String::new();
                        praeposition = self.combine_vectors(&praeposition, &definition_helper);
                        state = State::InRegex;
                    }
                    _ => {
                        let definition_helper = self.parse_regex(&current);
                        current = character.to_string();
                        praeposition = self.combine_vectors(&praeposition, &definition_helper);
                        state = State::ReadNextChar;
                    }
                },
                State::InRegex => match character {
                    ']' if last_char != '\\' => {
                        state = State::CheckForSubrulesRegex;
                    }
                    _ => current.push(character),
                },
            }
            last_char = character;
        }
        match state {
            State::CheckForSubrulesNormal => {
                let definition_helper = self.parse_definition(&current, name);
                praeposition = self.combine_vectors(&praeposition, &definition_helper);
                current = String::new();
            }
            State::CheckForSubrulesRegex => {
                let definition_helper = self.parse_regex(&current);
                praeposition = self.combine_vectors(&praeposition, &definition_helper);
                current = String::new();
            }
            _ => {}
        }
        if praeposition.len() == 0 {
            definitions.push(current.clone());
        }
        for definition in praeposition {
            definitions.push(definition + &current);
        }
        return definitions;
    }

    //
    fn apply_plus_subrule(&mut self, mut definition_helper: Vec<String>, name: &str) -> String {
        let mut new_rule_name = name.to_string();
        loop {
            new_rule_name.push('1');
            if !self.is_nonterm(&new_rule_name) {
                break;
            }
        }
        self.add_nonterm(new_rule_name.clone());
        for rule_definition in definition_helper.iter() {
            self.print_string(&new_rule_name, &rule_definition);
        }
        for def in definition_helper.iter_mut() {
            def.push_str(&(" ".to_string() + &new_rule_name + " "));
        }
        for rule_definition in definition_helper {
            self.print_string(&new_rule_name, &rule_definition);
        }
        return new_rule_name;
    }

    fn apply_mul_subrule(&mut self, mut definition_helper: Vec<String>, name: &str) -> String {
        let mut new_rule_name = name.to_string();
        loop {
            new_rule_name.push('1');
            if !self.is_nonterm(&new_rule_name) {
                break;
            }
        }
        self.add_nonterm(new_rule_name.clone());
        for def in definition_helper.iter_mut() {
            def.push_str(&(" ".to_string() + &new_rule_name + " "));
        }
        definition_helper.push(String::new());
        for rule_definition in definition_helper {
            self.print_string(&new_rule_name, &rule_definition);
        }
        return new_rule_name;
    }

    //This function combines two vectors. The resulting vector contains all combinations
    fn combine_vectors(&self, vec1: &Vec<String>, vec2: &Vec<String>) -> Vec<String> {
        if vec1.len() == 0 {
            return vec2.clone();
        }
        if vec2.len() == 0 {
            return vec1.clone();
        }
        let mut result_vec: Vec<String> = vec![];
        for string1 in vec1.iter() {
            for string2 in vec2.iter() {
                let mut help_string = string1.clone();
                help_string.push_str(string2);
                result_vec.push(help_string);
            }
        }
        return result_vec;
    }

    //Parse regex
    fn parse_regex(&self, string: &str) -> Vec<String> {
        let mut return_vec: Vec<String> = vec![];
        let mut last_char = 'a';
        let mut in_range = false;
        let mut from = ' ';
        for character in string.chars() {
            match character {
                '-' if last_char != '\\' => {
                    in_range = true;
                    from = last_char;
                }
                'n' if last_char == '\\' => {
                    return_vec.push(String::from("\n"));
                }
                'r' if last_char == '\\' => {
                    return_vec.push(String::from("\r"));
                }
                't' if last_char == '\\' => {
                    return_vec.push(String::from("\t"));
                }
                'f' if last_char == '\\' => {
                    return_vec.push(String::from("\x0C"));
                }
                '"' => {
                    return_vec.push(String::from("\""));
                }
                '{' => {
                    return_vec.push(String::from("&escaped_curly_bracked_open"));
                }
                '}' => {
                    return_vec.push(String::from("&escaped_curly_bracked_closed"));
                }
                '\\' if last_char == '\\' => {
                    return_vec.push(String::from("\\"));
                }
                _ => {
                    if in_range {
                        for x in (from as u32) + 1..(character as u32) + 1 {
                            return_vec.push(
                                char::from_u32(x)
                                    .expect("u32 could not be converted to char")
                                    .to_string(),
                            );
                        }
                        in_range = false;
                    } else {
                        return_vec.push(character.to_string());
                    }
                }
            }
            last_char = character;
        }

        return return_vec;
    }

    fn print_string(&mut self, name: &str, def: &str) {
        let mut string = def.to_string();
        string = self.rename_nterms(&string);
        string = string.replace("&escaped_single_quote", "'");
        string = string.replace("&escaped_backslash", "\\\\");
        string = string.trim().to_string();
        string = string.replace("&space", " ");
        string = string.replace("&escaped_n", "\n");
        string = string.replace("&escaped_r", "\r");
        string = string.replace("&escaped_curly_bracked_open", "\\{");
        string = string.replace("&escaped_curly_bracked_closed", "\\}");
        string = string.replace("&escaped_semicolon", ";");
        let name = self.replace_with_new_name(name.trim()).to_string();
        self.rules.push((name, string));
    }

    fn replace_with_new_name(&self, string: &str) -> &str {
        for nt in &self.nonterminals {
            if nt.0 == string {
                return &nt.1;
            }
        }
        panic!(
            "replace_with_name received a string that was no nonterminal: {}",
            string
        ); //should never happen
    }

    //This function searches a string for nonterms, places {} around them and replaces their names with their upper_case names
    fn rename_nterms(&self, string: &str) -> String {
        let mut result = String::new();
        for word in string.split_whitespace() {
            // print!("{}: ", word);
            if self.is_nonterm(word) {
                result.push('{');
                result.push_str(self.replace_with_new_name(word));
                result.push_str("}");
            // print!("is nterm!\n");
            // result.push(' ');
            } else {
                result.push_str(word);
                // print!("is NOT nterm!\n");
                result.push(' ');
            }
        }
        return result.trim().to_string();
    }

    //This function returns all possible characters
    #[allow(dead_code)]
    fn all_chars(&self) -> Vec<String> {
        let mut chars = vec![];
        for character in 0..0xD800 {
            if char::from_u32(character).expect("RAND_2188530336") == '{' {
                chars.push(String::from("&escaped_curly_bracked_open"));
            } else if char::from_u32(character).expect("RAND_2728928412") == '}' {
                chars.push(String::from("&escaped_curly_bracked_closed"));
            } else {
                chars.push(
                    char::from_u32(character)
                        .expect("RAND_3329310204")
                        .to_string(),
                );
            }
        }
        for character in 0xE000..0x11000 {
            if char::from_u32(character).expect("RAND_1752472326") == '{' {
                chars.push(String::from("&escaped_curly_bracked_open"));
            } else if char::from_u32(character).expect("RAND_1794224051") == '}' {
                chars.push(String::from("&escaped_curly_bracked_closed"));
            } else {
                chars.push(
                    char::from_u32(character)
                        .expect("RAND_3621278981")
                        .to_string(),
                );
            }
        }
        return chars;
    }

    //This function decodes all unicode characters
    fn replace_unicode(&self, string: String) -> String {
        let mut return_string = String::new();
        let mut last_char = ' ';
        let mut found_unicode = 0;
        let mut current_unicode = 0;
        for character in string.chars() {
            if found_unicode == 0 {
                match character {
                    'u' if last_char == '\\' => {
                        found_unicode = 4;
                    }
                    '\\' if last_char != '\\' => {}
                    '\\' => {
                        return_string.push('\\');
                    }
                    _ if last_char == '\\' => {
                        return_string.push('\\');
                        return_string.push(character);
                    }
                    _ => {
                        return_string.push(character);
                    }
                }
            } else {
                current_unicode =
                    current_unicode * 16 + character.to_digit(16).expect("RAND_1674350288");
                found_unicode = found_unicode - 1;
                if found_unicode == 0 {
                    //println!("{:x}", current_unicode);
                    return_string.push(char::from_u32(current_unicode).expect("RAND_1605623195"));
                    current_unicode = 0;
                }
            }
            last_char = character;
        }
        return return_string;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::prelude::*;

    #[test]
    fn check_is_nonterm_function() {
        let mut my_parser = AntlrParser {
            nonterminals: vec![],
            rules: vec![],
        };
        let file_path = "/tmp/tmp_grammar1.g4";
        let mut file = File::create(file_path).expect("Could not create file");
        file.write_all(
            b"grammar test;\nprog: 'test';\n LITERAL: 'test2';\nBREAK: 'test3';\nBla: 'test4';",
        ).expect("Could not write to file");
        my_parser.parse_antlr_grammar(file_path);
        assert!(my_parser.is_nonterm("prog"));
        assert!(my_parser.is_nonterm("LITERAL"));
        assert!(my_parser.is_nonterm("BREAK"));
        assert!(my_parser.is_nonterm("Bla"));
        fs::remove_file(file_path).expect("Could not remove file");
    }

    #[test]
    fn check_combine_vectors_function() {
        let my_parser = AntlrParser {
            nonterminals: vec![],
            rules: vec![],
        };
        let vec = my_parser.combine_vectors(
            &vec!["test".to_string(), "a".to_string()],
            &vec!["lulz".to_string(), "b".to_string()],
        );
        assert_eq!(vec, vec!["testlulz", "testb", "alulz", "ab"]);
    }

    #[test]
    fn check_rename_nterms_function() {
        let mut my_parser = AntlrParser {
            nonterminals: vec![],
            rules: vec![],
        };
        let file_path = "/tmp/tmp_grammar2.g4";
        let mut file = File::create(file_path).expect("Could not create file");
        file.write_all(b"grammar test;\nprog: 'test';\n LITERAL: 'test2';")
            .expect("Could not write to file");
        my_parser.parse_antlr_grammar(file_path);
        let string = my_parser.rename_nterms("prog asfas LITERALasd LITERAL");
        assert_eq!(string, "{PROG}asfas LITERALasd {LITERAL}");
        fs::remove_file(file_path).expect("Could not remove file");
    }

}
