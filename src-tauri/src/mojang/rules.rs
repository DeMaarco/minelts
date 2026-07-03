use super::manifest::{ArgumentValue, Library, Rule};

const OS_NAME: &str = "windows";
const OS_ARCH: &str = "x86_64";

pub fn rule_matches(rule: &Rule) -> bool {
    if let Some(os) = &rule.os {
        if let Some(name) = &os.name {
            if name != OS_NAME {
                return false;
            }
        }
        if let Some(arch) = &os.arch {
            if arch != OS_ARCH {
                return false;
            }
        }
    }
    if let Some(features) = &rule.features {
        for (feature, required) in features {
            let present = match feature.as_str() {
                "is_demo_user" | "has_custom_resolution" | "has_quick_plays_support"
                | "is_quick_play_singleplayer" | "is_quick_play_multiplayer"
                | "is_quick_play_realms" => false,
                _ => false,
            };
            if present != *required {
                return false;
            }
        }
    }
    true
}

/// Evalúa reglas de una librería/argumento. Sin reglas => incluir.
pub fn should_include(rules: Option<&[Rule]>) -> bool {
    let Some(rules) = rules else {
        return true;
    };

    let mut allow = false;
    for rule in rules {
        if rule_matches(rule) {
            allow = rule.action == "allow";
        }
    }
    allow
}

pub fn library_applies(library: &Library) -> bool {
    should_include(library.rules.as_deref())
}

pub fn resolve_argument_values(args: &[ArgumentValue]) -> Vec<String> {
    let mut result = Vec::new();
    for arg in args {
        match arg {
            ArgumentValue::Plain(s) => result.push(s.clone()),
            ArgumentValue::Conditional { rules, value } => {
                if should_include(Some(rules)) {
                    result.extend(value.clone().into_vec());
                }
            }
        }
    }
    result
}

/// Clave de natives para Windows x64.
pub fn native_classifier_key() -> &'static str {
    "natives-windows"
}

/// Clave de natives para Windows x64 en versiones recientes (arm64 variant).
pub fn native_classifier_key_alt() -> &'static str {
    "natives-windows-x86_64"
}
