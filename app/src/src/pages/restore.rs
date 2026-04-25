//! Restore wallet — 4-step wizard: phrase input → set PIN → confirm PIN → success.
//!
//! On PIN confirmation the wizard calls `import_wallet_from_mnemonic` with the
//! collected phrase and PIN as the password. A mismatch between the two PIN
//! entries shakes the dots and clears the confirm field. A backend error (bad
//! phrase) shakes and returns the user to Step 1 with an error message.
//! On success the wizard parks on a confirmation step until the user taps
//! Continue, which flips `auth_state` to `Unlocked` and navigates to home.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use rustok_types::WalletInfo;
use serde::Serialize;

use crate::app::WalletState;
use crate::bridge::tauri_invoke;
use crate::components::{Keypad, PasscodeDots, WizardSuccess, PASSCODE_LENGTH};
use wasm_bindgen::JsCast;

// ─── Token constants (new palette) ──────────────────────────────────────────
const BG: &str = "#F6F7FB";
const BRAND: &str = "#0A1123";
const ACCENT: &str = "#8387C3";
const MUTED: &str = "#959BB5";
const SUCCESS: &str = "#4AB37B";
const SURFACE_BORDER: &str = "#E4E6F0";
const FONT: &str =
    r#"Roboto, -apple-system, "SF Pro Display", "SF Pro Text", system-ui, sans-serif"#;
const MONO: &str = r#""Roboto Mono", "SF Mono", ui-monospace, monospace"#;

// BIP-39 English wordlist (2048 words) — compiled into the WASM bundle for
// synchronous prefix-matching autocomplete. ~13 KB; trivial compared to the
// crypto assets already in the binary.
const BIP39_WORDS: &[&str; 2048] = &[
    "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd", "abuse", "access", "accident", "account", "accuse", "achieve", "acid",
    "acoustic", "acquire", "across", "act", "action", "actor", "actress", "actual", "adapt", "add", "addict", "address", "adjust", "admit", "adult", "advance",
    "advice", "aerobic", "affair", "afford", "afraid", "again", "age", "agent", "agree", "ahead", "aim", "air", "airport", "aisle", "alarm", "album",
    "alcohol", "alert", "alien", "all", "alley", "allow", "almost", "alone", "alpha", "already", "also", "alter", "always", "amateur", "amazing", "among",
    "amount", "amused", "analyst", "anchor", "ancient", "anger", "angle", "angry", "animal", "ankle", "announce", "annual", "another", "answer", "antenna", "antique",
    "anxiety", "any", "apart", "apology", "appear", "apple", "approve", "april", "arch", "arctic", "area", "arena", "argue", "arm", "armed", "armor",
    "army", "around", "arrange", "arrest", "arrive", "arrow", "art", "artefact", "artist", "artwork", "ask", "aspect", "assault", "asset", "assist", "assume",
    "asthma", "athlete", "atom", "attack", "attend", "attitude", "attract", "auction", "audit", "august", "aunt", "author", "auto", "autumn", "average", "avocado",
    "avoid", "awake", "aware", "away", "awesome", "awful", "awkward", "axis", "baby", "bachelor", "bacon", "badge", "bag", "balance", "balcony", "ball",
    "bamboo", "banana", "banner", "bar", "barely", "bargain", "barrel", "base", "basic", "basket", "battle", "beach", "bean", "beauty", "because", "become",
    "beef", "before", "begin", "behave", "behind", "believe", "below", "belt", "bench", "benefit", "best", "betray", "better", "between", "beyond", "bicycle",
    "bid", "bike", "bind", "biology", "bird", "birth", "bitter", "black", "blade", "blame", "blanket", "blast", "bleak", "bless", "blind", "blood",
    "blossom", "blouse", "blue", "blur", "blush", "board", "boat", "body", "boil", "bomb", "bone", "bonus", "book", "boost", "border", "boring",
    "borrow", "boss", "bottom", "bounce", "box", "boy", "bracket", "brain", "brand", "brass", "brave", "bread", "breeze", "brick", "bridge", "brief",
    "bright", "bring", "brisk", "broccoli", "broken", "bronze", "broom", "brother", "brown", "brush", "bubble", "buddy", "budget", "buffalo", "build", "bulb",
    "bulk", "bullet", "bundle", "bunker", "burden", "burger", "burst", "bus", "business", "busy", "butter", "buyer", "buzz", "cabbage", "cabin", "cable",
    "cactus", "cage", "cake", "call", "calm", "camera", "camp", "can", "canal", "cancel", "candy", "cannon", "canoe", "canvas", "canyon", "capable",
    "capital", "captain", "car", "carbon", "card", "cargo", "carpet", "carry", "cart", "case", "cash", "casino", "castle", "casual", "cat", "catalog",
    "catch", "category", "cattle", "caught", "cause", "caution", "cave", "ceiling", "celery", "cement", "census", "century", "cereal", "certain", "chair", "chalk",
    "champion", "change", "chaos", "chapter", "charge", "chase", "chat", "cheap", "check", "cheese", "chef", "cherry", "chest", "chicken", "chief", "child",
    "chimney", "choice", "choose", "chronic", "chuckle", "chunk", "churn", "cigar", "cinnamon", "circle", "citizen", "city", "civil", "claim", "clap", "clarify",
    "claw", "clay", "clean", "clerk", "clever", "click", "client", "cliff", "climb", "clinic", "clip", "clock", "clog", "close", "cloth", "cloud",
    "clown", "club", "clump", "cluster", "clutch", "coach", "coast", "coconut", "code", "coffee", "coil", "coin", "collect", "color", "column", "combine",
    "come", "comfort", "comic", "common", "company", "concert", "conduct", "confirm", "congress", "connect", "consider", "control", "convince", "cook", "cool", "copper",
    "copy", "coral", "core", "corn", "correct", "cost", "cotton", "couch", "country", "couple", "course", "cousin", "cover", "coyote", "crack", "cradle",
    "craft", "cram", "crane", "crash", "crater", "crawl", "crazy", "cream", "credit", "creek", "crew", "cricket", "crime", "crisp", "critic", "crop",
    "cross", "crouch", "crowd", "crucial", "cruel", "cruise", "crumble", "crunch", "crush", "cry", "crystal", "cube", "culture", "cup", "cupboard", "curious",
    "current", "curtain", "curve", "cushion", "custom", "cute", "cycle", "dad", "damage", "damp", "dance", "danger", "daring", "dash", "daughter", "dawn",
    "day", "deal", "debate", "debris", "decade", "december", "decide", "decline", "decorate", "decrease", "deer", "defense", "define", "defy", "degree", "delay",
    "deliver", "demand", "demise", "denial", "dentist", "deny", "depart", "depend", "deposit", "depth", "deputy", "derive", "describe", "desert", "design", "desk",
    "despair", "destroy", "detail", "detect", "develop", "device", "devote", "diagram", "dial", "diamond", "diary", "dice", "diesel", "diet", "differ", "digital",
    "dignity", "dilemma", "dinner", "dinosaur", "direct", "dirt", "disagree", "discover", "disease", "dish", "dismiss", "disorder", "display", "distance", "divert", "divide",
    "divorce", "dizzy", "doctor", "document", "dog", "doll", "dolphin", "domain", "donate", "donkey", "donor", "door", "dose", "double", "dove", "draft",
    "dragon", "drama", "drastic", "draw", "dream", "dress", "drift", "drill", "drink", "drip", "drive", "drop", "drum", "dry", "duck", "dumb",
    "dune", "during", "dust", "dutch", "duty", "dwarf", "dynamic", "eager", "eagle", "early", "earn", "earth", "easily", "east", "easy", "echo",
    "ecology", "economy", "edge", "edit", "educate", "effort", "egg", "eight", "either", "elbow", "elder", "electric", "elegant", "element", "elephant", "elevator",
    "elite", "else", "embark", "embody", "embrace", "emerge", "emotion", "employ", "empower", "empty", "enable", "enact", "end", "endless", "endorse", "enemy",
    "energy", "enforce", "engage", "engine", "enhance", "enjoy", "enlist", "enough", "enrich", "enroll", "ensure", "enter", "entire", "entry", "envelope", "episode",
    "equal", "equip", "era", "erase", "erode", "erosion", "error", "erupt", "escape", "essay", "essence", "estate", "eternal", "ethics", "evidence", "evil",
    "evoke", "evolve", "exact", "example", "excess", "exchange", "excite", "exclude", "excuse", "execute", "exercise", "exhaust", "exhibit", "exile", "exist", "exit",
    "exotic", "expand", "expect", "expire", "explain", "expose", "express", "extend", "extra", "eye", "eyebrow", "fabric", "face", "faculty", "fade", "faint",
    "faith", "fall", "false", "fame", "family", "famous", "fan", "fancy", "fantasy", "farm", "fashion", "fat", "fatal", "father", "fatigue", "fault",
    "favorite", "feature", "february", "federal", "fee", "feed", "feel", "female", "fence", "festival", "fetch", "fever", "few", "fiber", "fiction", "field",
    "figure", "file", "film", "filter", "final", "find", "fine", "finger", "finish", "fire", "firm", "first", "fiscal", "fish", "fit", "fitness",
    "fix", "flag", "flame", "flash", "flat", "flavor", "flee", "flight", "flip", "float", "flock", "floor", "flower", "fluid", "flush", "fly",
    "foam", "focus", "fog", "foil", "fold", "follow", "food", "foot", "force", "forest", "forget", "fork", "fortune", "forum", "forward", "fossil",
    "foster", "found", "fox", "fragile", "frame", "frequent", "fresh", "friend", "fringe", "frog", "front", "frost", "frown", "frozen", "fruit", "fuel",
    "fun", "funny", "furnace", "fury", "future", "gadget", "gain", "galaxy", "gallery", "game", "gap", "garage", "garbage", "garden", "garlic", "garment",
    "gas", "gasp", "gate", "gather", "gauge", "gaze", "general", "genius", "genre", "gentle", "genuine", "gesture", "ghost", "giant", "gift", "giggle",
    "ginger", "giraffe", "girl", "give", "glad", "glance", "glare", "glass", "glide", "glimpse", "globe", "gloom", "glory", "glove", "glow", "glue",
    "goat", "goddess", "gold", "good", "goose", "gorilla", "gospel", "gossip", "govern", "gown", "grab", "grace", "grain", "grant", "grape", "grass",
    "gravity", "great", "green", "grid", "grief", "grit", "grocery", "group", "grow", "grunt", "guard", "guess", "guide", "guilt", "guitar", "gun",
    "gym", "habit", "hair", "half", "hammer", "hamster", "hand", "happy", "harbor", "hard", "harsh", "harvest", "hat", "have", "hawk", "hazard",
    "head", "health", "heart", "heavy", "hedgehog", "height", "hello", "helmet", "help", "hen", "hero", "hidden", "high", "hill", "hint", "hip",
    "hire", "history", "hobby", "hockey", "hold", "hole", "holiday", "hollow", "home", "honey", "hood", "hope", "horn", "horror", "horse", "hospital",
    "host", "hotel", "hour", "hover", "hub", "huge", "human", "humble", "humor", "hundred", "hungry", "hunt", "hurdle", "hurry", "hurt", "husband",
    "hybrid", "ice", "icon", "idea", "identify", "idle", "ignore", "ill", "illegal", "illness", "image", "imitate", "immense", "immune", "impact", "impose",
    "improve", "impulse", "inch", "include", "income", "increase", "index", "indicate", "indoor", "industry", "infant", "inflict", "inform", "inhale", "inherit", "initial",
    "inject", "injury", "inmate", "inner", "innocent", "input", "inquiry", "insane", "insect", "inside", "inspire", "install", "intact", "interest", "into", "invest",
    "invite", "involve", "iron", "island", "isolate", "issue", "item", "ivory", "jacket", "jaguar", "jar", "jazz", "jealous", "jeans", "jelly", "jewel",
    "job", "join", "joke", "journey", "joy", "judge", "juice", "jump", "jungle", "junior", "junk", "just", "kangaroo", "keen", "keep", "ketchup",
    "key", "kick", "kid", "kidney", "kind", "kingdom", "kiss", "kit", "kitchen", "kite", "kitten", "kiwi", "knee", "knife", "knock", "know",
    "lab", "label", "labor", "ladder", "lady", "lake", "lamp", "language", "laptop", "large", "later", "latin", "laugh", "laundry", "lava", "law",
    "lawn", "lawsuit", "layer", "lazy", "leader", "leaf", "learn", "leave", "lecture", "left", "leg", "legal", "legend", "leisure", "lemon", "lend",
    "length", "lens", "leopard", "lesson", "letter", "level", "liar", "liberty", "library", "license", "life", "lift", "light", "like", "limb", "limit",
    "link", "lion", "liquid", "list", "little", "live", "lizard", "load", "loan", "lobster", "local", "lock", "logic", "lonely", "long", "loop",
    "lottery", "loud", "lounge", "love", "loyal", "lucky", "luggage", "lumber", "lunar", "lunch", "luxury", "lyrics", "machine", "mad", "magic", "magnet",
    "maid", "mail", "main", "major", "make", "mammal", "man", "manage", "mandate", "mango", "mansion", "manual", "maple", "marble", "march", "margin",
    "marine", "market", "marriage", "mask", "mass", "master", "match", "material", "math", "matrix", "matter", "maximum", "maze", "meadow", "mean", "measure",
    "meat", "mechanic", "medal", "media", "melody", "melt", "member", "memory", "mention", "menu", "mercy", "merge", "merit", "merry", "mesh", "message",
    "metal", "method", "middle", "midnight", "milk", "million", "mimic", "mind", "minimum", "minor", "minute", "miracle", "mirror", "misery", "miss", "mistake",
    "mix", "mixed", "mixture", "mobile", "model", "modify", "mom", "moment", "monitor", "monkey", "monster", "month", "moon", "moral", "more", "morning",
    "mosquito", "mother", "motion", "motor", "mountain", "mouse", "move", "movie", "much", "muffin", "mule", "multiply", "muscle", "museum", "mushroom", "music",
    "must", "mutual", "myself", "mystery", "myth", "naive", "name", "napkin", "narrow", "nasty", "nation", "nature", "near", "neck", "need", "negative",
    "neglect", "neither", "nephew", "nerve", "nest", "net", "network", "neutral", "never", "news", "next", "nice", "night", "noble", "noise", "nominee",
    "noodle", "normal", "north", "nose", "notable", "note", "nothing", "notice", "novel", "now", "nuclear", "number", "nurse", "nut", "oak", "obey",
    "object", "oblige", "obscure", "observe", "obtain", "obvious", "occur", "ocean", "october", "odor", "off", "offer", "office", "often", "oil", "okay",
    "old", "olive", "olympic", "omit", "once", "one", "onion", "online", "only", "open", "opera", "opinion", "oppose", "option", "orange", "orbit",
    "orchard", "order", "ordinary", "organ", "orient", "original", "orphan", "ostrich", "other", "outdoor", "outer", "output", "outside", "oval", "oven", "over",
    "own", "owner", "oxygen", "oyster", "ozone", "pact", "paddle", "page", "pair", "palace", "palm", "panda", "panel", "panic", "panther", "paper",
    "parade", "parent", "park", "parrot", "party", "pass", "patch", "path", "patient", "patrol", "pattern", "pause", "pave", "payment", "peace", "peanut",
    "pear", "peasant", "pelican", "pen", "penalty", "pencil", "people", "pepper", "perfect", "permit", "person", "pet", "phone", "photo", "phrase", "physical",
    "piano", "picnic", "picture", "piece", "pig", "pigeon", "pill", "pilot", "pink", "pioneer", "pipe", "pistol", "pitch", "pizza", "place", "planet",
    "plastic", "plate", "play", "please", "pledge", "pluck", "plug", "plunge", "poem", "poet", "point", "polar", "pole", "police", "pond", "pony",
    "pool", "popular", "portion", "position", "possible", "post", "potato", "pottery", "poverty", "powder", "power", "practice", "praise", "predict", "prefer", "prepare",
    "present", "pretty", "prevent", "price", "pride", "primary", "print", "priority", "prison", "private", "prize", "problem", "process", "produce", "profit", "program",
    "project", "promote", "proof", "property", "prosper", "protect", "proud", "provide", "public", "pudding", "pull", "pulp", "pulse", "pumpkin", "punch", "pupil",
    "puppy", "purchase", "purity", "purpose", "purse", "push", "put", "puzzle", "pyramid", "quality", "quantum", "quarter", "question", "quick", "quit", "quiz",
    "quote", "rabbit", "raccoon", "race", "rack", "radar", "radio", "rail", "rain", "raise", "rally", "ramp", "ranch", "random", "range", "rapid",
    "rare", "rate", "rather", "raven", "raw", "razor", "ready", "real", "reason", "rebel", "rebuild", "recall", "receive", "recipe", "record", "recycle",
    "reduce", "reflect", "reform", "refuse", "region", "regret", "regular", "reject", "relax", "release", "relief", "rely", "remain", "remember", "remind", "remove",
    "render", "renew", "rent", "reopen", "repair", "repeat", "replace", "report", "require", "rescue", "resemble", "resist", "resource", "response", "result", "retire",
    "retreat", "return", "reunion", "reveal", "review", "reward", "rhythm", "rib", "ribbon", "rice", "rich", "ride", "ridge", "rifle", "right", "rigid",
    "ring", "riot", "ripple", "risk", "ritual", "rival", "river", "road", "roast", "robot", "robust", "rocket", "romance", "roof", "rookie", "room",
    "rose", "rotate", "rough", "round", "route", "royal", "rubber", "rude", "rug", "rule", "run", "runway", "rural", "sad", "saddle", "sadness",
    "safe", "sail", "salad", "salmon", "salon", "salt", "salute", "same", "sample", "sand", "satisfy", "satoshi", "sauce", "sausage", "save", "say",
    "scale", "scan", "scare", "scatter", "scene", "scheme", "school", "science", "scissors", "scorpion", "scout", "scrap", "screen", "script", "scrub", "sea",
    "search", "season", "seat", "second", "secret", "section", "security", "seed", "seek", "segment", "select", "sell", "seminar", "senior", "sense", "sentence",
    "series", "service", "session", "settle", "setup", "seven", "shadow", "shaft", "shallow", "share", "shed", "shell", "sheriff", "shield", "shift", "shine",
    "ship", "shiver", "shock", "shoe", "shoot", "shop", "short", "shoulder", "shove", "shrimp", "shrug", "shuffle", "shy", "sibling", "sick", "side",
    "siege", "sight", "sign", "silent", "silk", "silly", "silver", "similar", "simple", "since", "sing", "siren", "sister", "situate", "six", "size",
    "skate", "sketch", "ski", "skill", "skin", "skirt", "skull", "slab", "slam", "sleep", "slender", "slice", "slide", "slight", "slim", "slogan",
    "slot", "slow", "slush", "small", "smart", "smile", "smoke", "smooth", "snack", "snake", "snap", "sniff", "snow", "soap", "soccer", "social",
    "sock", "soda", "soft", "solar", "soldier", "solid", "solution", "solve", "someone", "song", "soon", "sorry", "sort", "soul", "sound", "soup",
    "source", "south", "space", "spare", "spatial", "spawn", "speak", "special", "speed", "spell", "spend", "sphere", "spice", "spider", "spike", "spin",
    "spirit", "split", "spoil", "sponsor", "spoon", "sport", "spot", "spray", "spread", "spring", "spy", "square", "squeeze", "squirrel", "stable", "stadium",
    "staff", "stage", "stairs", "stamp", "stand", "start", "state", "stay", "steak", "steel", "stem", "step", "stereo", "stick", "still", "sting",
    "stock", "stomach", "stone", "stool", "story", "stove", "strategy", "street", "strike", "strong", "struggle", "student", "stuff", "stumble", "style", "subject",
    "submit", "subway", "success", "such", "sudden", "suffer", "sugar", "suggest", "suit", "summer", "sun", "sunny", "sunset", "super", "supply", "supreme",
    "sure", "surface", "surge", "surprise", "surround", "survey", "suspect", "sustain", "swallow", "swamp", "swap", "swarm", "swear", "sweet", "swift", "swim",
    "swing", "switch", "sword", "symbol", "symptom", "syrup", "system", "table", "tackle", "tag", "tail", "talent", "talk", "tank", "tape", "target",
    "task", "taste", "tattoo", "taxi", "teach", "team", "tell", "ten", "tenant", "tennis", "tent", "term", "test", "text", "thank", "that",
    "theme", "then", "theory", "there", "they", "thing", "this", "thought", "three", "thrive", "throw", "thumb", "thunder", "ticket", "tide", "tiger",
    "tilt", "timber", "time", "tiny", "tip", "tired", "tissue", "title", "toast", "tobacco", "today", "toddler", "toe", "together", "toilet", "token",
    "tomato", "tomorrow", "tone", "tongue", "tonight", "tool", "tooth", "top", "topic", "topple", "torch", "tornado", "tortoise", "toss", "total", "tourist",
    "toward", "tower", "town", "toy", "track", "trade", "traffic", "tragic", "train", "transfer", "trap", "trash", "travel", "tray", "treat", "tree",
    "trend", "trial", "tribe", "trick", "trigger", "trim", "trip", "trophy", "trouble", "truck", "true", "truly", "trumpet", "trust", "truth", "try",
    "tube", "tuition", "tumble", "tuna", "tunnel", "turkey", "turn", "turtle", "twelve", "twenty", "twice", "twin", "twist", "two", "type", "typical",
    "ugly", "umbrella", "unable", "unaware", "uncle", "uncover", "under", "undo", "unfair", "unfold", "unhappy", "uniform", "unique", "unit", "universe", "unknown",
    "unlock", "until", "unusual", "unveil", "update", "upgrade", "uphold", "upon", "upper", "upset", "urban", "urge", "usage", "use", "used", "useful",
    "useless", "usual", "utility", "vacant", "vacuum", "vague", "valid", "valley", "valve", "van", "vanish", "vapor", "various", "vast", "vault", "vehicle",
    "velvet", "vendor", "venture", "venue", "verb", "verify", "version", "very", "vessel", "veteran", "viable", "vibrant", "vicious", "victory", "video", "view",
    "village", "vintage", "violin", "virtual", "virus", "visa", "visit", "visual", "vital", "vivid", "vocal", "voice", "void", "volcano", "volume", "vote",
    "voyage", "wage", "wagon", "wait", "walk", "wall", "walnut", "want", "warfare", "warm", "warrior", "wash", "wasp", "waste", "water", "wave",
    "way", "wealth", "weapon", "wear", "weasel", "weather", "web", "wedding", "weekend", "weird", "welcome", "west", "wet", "whale", "what", "wheat",
    "wheel", "when", "where", "whip", "whisper", "wide", "width", "wife", "wild", "will", "win", "window", "wine", "wing", "wink", "winner",
    "winter", "wire", "wisdom", "wise", "wish", "witness", "wolf", "woman", "wonder", "wood", "wool", "word", "work", "world", "worry", "worth",
    "wrap", "wreck", "wrestle", "wrist", "write", "wrong", "yard", "year", "yellow", "you", "young", "youth", "zebra", "zero", "zone", "zoo",
];

// ─── Tauri arg type ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ImportArgs {
    phrase: String,
    password: String,
}

// ─── Wizard step ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Phrase,
    SetPin,
    ConfirmPin,
    Success,
}

/// Restore wallet component.
#[component]
pub fn RestorePage() -> impl IntoView {
    let auth_state = use_context::<RwSignal<WalletState>>()
        .expect("WalletState context missing — must be provided in App");
    let navigate = use_navigate();

    let step = RwSignal::new(Step::Phrase);
    let phrase = RwSignal::new(String::new());
    let phrase_error = RwSignal::new(Option::<String>::None);
    let pin = RwSignal::new(String::new());
    let confirm_pin = RwSignal::new(String::new());
    let shake = RwSignal::new(false);
    let error = RwSignal::new(false);
    let loading = RwSignal::new(false);
    let textarea_ref = NodeRef::new();

    let phrase_valid = Signal::derive(move || {
        let count = phrase.read().trim().split_whitespace().count();
        matches!(count, 12 | 15 | 18 | 21 | 24)
    });

    let suggestions = Signal::derive(move || {
        let text = phrase.get();
        let current = if text.ends_with(char::is_whitespace) || text.is_empty() {
            ""
        } else {
            text.split_whitespace().last().unwrap_or("")
        };

        if current.is_empty() {
            return Vec::<&'static str>::new();
        }

        let prefix = current.to_lowercase();
        let matches: Vec<&'static str> = BIP39_WORDS
            .iter()
            .filter(|&&w| w.starts_with(&prefix))
            .take(6)
            .copied()
            .collect();

        // Hide when the current word is already an exact match in the wordlist.
        if BIP39_WORDS.iter().any(|&w| w == prefix) {
            return Vec::new();
        }

        matches
    });

    let filled_set = Signal::derive(move || pin.read().len());
    let filled_confirm = Signal::derive(move || confirm_pin.read().len());

    // Step 1 "Back" needs its own navigate clone (view! will move it).
    let nav_back = navigate.clone();

    // Step 1 → 2.
    let go_to_set_pin = move |_| {
        phrase_error.set(None);
        pin.set(String::new());
        step.set(Step::SetPin);
    };

    // Step 2 keypad handlers.
    let on_set_press = Callback::new(move |d: char| {
        let mut s = pin.get_untracked();
        if s.len() < PASSCODE_LENGTH {
            s.push(d);
        }
        pin.set(s.clone());
        if s.len() == PASSCODE_LENGTH {
            confirm_pin.set(String::new());
            step.set(Step::ConfirmPin);
        }
    });

    let on_set_back = Callback::new(move |_: ()| {
        if pin.with_untracked(|s| s.is_empty()) {
            step.set(Step::Phrase);
        } else {
            pin.update(|s| {
                s.pop();
            });
        }
    });

    // Step 3 — confirm PIN, then import wallet.
    let do_restore = move |confirmed: String| {
        loading.set(true);
        spawn_local(async move {
            match tauri_invoke::<_, WalletInfo>(
                "import_wallet_from_mnemonic",
                &ImportArgs {
                    phrase: phrase.get_untracked().trim().to_string(),
                    password: confirmed,
                },
            )
            .await
            {
                // Defer auth_state + navigate until the user taps Continue
                // on the Success step — keeps the wizard symmetric with
                // Create and gives the user a clear "all done" beat.
                Ok(_) => {
                    loading.set(false);
                    step.set(Step::Success);
                }
                Err(e) => {
                    // Phrase rejected by backend — shake, then return to Step 1.
                    phrase_error.set(Some(e));
                    error.set(true);
                    shake.set(true);
                    set_timeout(
                        move || {
                            confirm_pin.set(String::new());
                            pin.set(String::new());
                            error.set(false);
                            shake.set(false);
                            loading.set(false);
                            step.set(Step::Phrase);
                        },
                        std::time::Duration::from_millis(500),
                    );
                }
            }
        });
    };

    let go_home = {
        let navigate = navigate.clone();
        move |_| {
            auth_state.set(WalletState::Unlocked);
            navigate("/", Default::default());
        }
    };

    let on_confirm_press = Callback::new(move |d: char| {
        if loading.get_untracked() || shake.get_untracked() {
            return;
        }
        let mut s = confirm_pin.get_untracked();
        if s.len() < PASSCODE_LENGTH {
            s.push(d);
        }
        let len = s.len();
        confirm_pin.set(s.clone());
        if len == PASSCODE_LENGTH {
            if s == pin.get_untracked() {
                do_restore.clone()(s);
            } else {
                error.set(true);
                shake.set(true);
                set_timeout(
                    move || {
                        confirm_pin.set(String::new());
                        error.set(false);
                        shake.set(false);
                    },
                    std::time::Duration::from_millis(500),
                );
            }
        }
    });

    let on_confirm_back = Callback::new(move |_: ()| {
        if loading.get_untracked() || shake.get_untracked() {
            return;
        }
        if confirm_pin.with_untracked(|s| s.is_empty()) {
            pin.set(String::new());
            step.set(Step::SetPin);
        } else {
            confirm_pin.update(|s| {
                s.pop();
            });
        }
    });

    view! {
        <div style=format!(
            "display:flex;flex-direction:column;\
             min-height:calc(100dvh - env(safe-area-inset-top) - env(safe-area-inset-bottom));\
             background:{BG};padding-top:52px;"
        )>

            // ── Step 1: Phrase input ─────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::Phrase { "flex" } else { "none" }
            )>
                <div style="padding:24px 24px 0;">
                    <button
                        on:click=move |_| { nav_back("/wallet/create", Default::default()); }
                        style=format!(
                            "background:none;border:none;padding:0;cursor:pointer;\
                             color:{MUTED};font-family:{FONT};font-size:15px;\
                             display:flex;align-items:center;gap:6px;"
                        )
                    >
                        <svg width="18" height="18" viewBox="0 0 24 24" fill="none"
                            stroke="currentColor" stroke-width="2"
                            stroke-linecap="round" stroke-linejoin="round">
                            <path d="M19 12H5M12 5l-7 7 7 7"/>
                        </svg>
                        "Back"
                    </button>

                    <div style=format!(
                        "margin-top:20px;font-family:{FONT};font-size:22px;\
                         font-weight:700;color:{BRAND};letter-spacing:-0.4px;"
                    )>"Restore wallet"</div>
                    <div style=format!(
                        "margin-top:6px;font-family:{FONT};font-size:14px;\
                         color:{MUTED};line-height:1.45;"
                    )>
                        "Paste or type your recovery phrase. Words are separated by spaces."
                    </div>
                </div>

                <div style="padding:20px 24px 0;flex:1;display:flex;flex-direction:column;">
                    <textarea
                        node_ref=textarea_ref
                        style=format!(
                            "width:100%;min-height:140px;padding:14px;\
                             background:#FFFFFF;border:1.5px solid {SURFACE_BORDER};\
                             border-radius:16px;font-family:{MONO};font-size:14px;\
                             color:{BRAND};resize:none;outline:none;\
                             box-sizing:border-box;line-height:1.55;"
                        )
                        placeholder="abandon ability able about above absent…"
                        autocapitalize="none"
                        spellcheck="false"
                        on:input=move |ev| {
                            phrase.set(event_target_value(&ev));
                            phrase_error.set(None);
                        }
                    />

                    // BIP-39 word suggestions
                    <div style=move || format!(
                        "display:{};gap:8px;overflow-x:auto;scrollbar-width:none;\
                         -ms-overflow-style:none;margin-top:12px;padding:4px 0;\
                         flex-wrap:nowrap;flex-direction:row;",
                        if suggestions.get().is_empty() { "none" } else { "flex" }
                    )>
                        {move || {
                            suggestions.get().into_iter().map(|word| {
                                let word_for_closure = word;
                                view! {
                                    <button
                                        on:click=move |_| {
                                            let current_text = phrase.get();
                                            let mut words: Vec<&str> = current_text.split_whitespace().collect();
                                            if !current_text.ends_with(char::is_whitespace) {
                                                words.pop();
                                            }
                                            words.push(word_for_closure);
                                            let new_phrase = words.join(" ") + " ";
                                            phrase.set(new_phrase.clone());
                                            phrase_error.set(None);
                                            if let Some(el) = textarea_ref.get() {
                                                if let Ok(el) = el.dyn_into::<web_sys::HtmlTextAreaElement>() {
                                                    let len = u32::try_from(new_phrase.len()).unwrap_or(0);
                                                    let _ = el.focus();
                                                    let _ = el.set_selection_range(len, len);
                                                }
                                            }
                                        }
                                        style=format!(
                                            "flex:0 0 auto;padding:6px 14px;border-radius:999px;\
                                             background:rgba(131,135,195,0.14);color:{};\
                                             font-family:{};font-size:14px;font-weight:500;\
                                             cursor:pointer;white-space:nowrap;border:none;\
                                             transition:background 0.15s;",
                                            BRAND, FONT
                                        )
                                    >
                                        {word}
                                    </button>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </div>

                    // Validity indicator
                    <div style="margin-top:10px;display:flex;align-items:center;gap:8px;">
                        <div style=move || format!(
                            "width:8px;height:8px;border-radius:50%;\
                             background:{};transition:background 0.15s;",
                            if phrase_valid.get() { SUCCESS } else { SURFACE_BORDER }
                        )/>
                        <span style=format!(
                            "font-family:{FONT};font-size:12px;\
                             color:{MUTED};font-weight:500;"
                        )>
                            {move || if phrase_valid.get() {
                                "Looks valid"
                            } else {
                                "12, 15, 18, 21 or 24 words needed"
                            }}
                        </span>
                    </div>

                    // Backend error banner
                    {move || phrase_error.get().map(|e| view! {
                        <div style=format!(
                            "margin-top:8px;font-family:{FONT};\
                             font-size:12px;color:#E06B6B;line-height:1.4;"
                        )>{e}</div>
                    })}
                </div>

                <div style="padding:16px 24px max(24px,env(safe-area-inset-bottom));">
                    <button
                        on:click=go_to_set_pin
                        disabled=move || !phrase_valid.get()
                        style=move || format!(
                            "width:100%;height:56px;border:none;border-radius:16px;\
                             font-family:{FONT};font-size:16px;font-weight:700;\
                             letter-spacing:-0.2px;cursor:pointer;color:#FFFFFF;\
                             background:{};transition:background 0.15s;",
                            if phrase_valid.get() { ACCENT } else { SURFACE_BORDER }
                        )
                    >
                        "Continue"
                    </button>
                </div>
            </div>

            // ── Step 2: Set PIN ─────────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::SetPin { "flex" } else { "none" }
            )>
                <div style="display:flex;flex-direction:column;align-items:center;padding:32px 24px 0;">
                    <div style=format!(
                        "width:72px;height:72px;border-radius:22px;\
                         background:rgba(131,135,195,0.12);\
                         display:flex;align-items:center;justify-content:center;\
                         color:{ACCENT};"
                    )>
                        <svg width="32" height="32" viewBox="0 0 24 24" fill="none"
                            stroke="currentColor" stroke-width="1.8"
                            stroke-linecap="round" stroke-linejoin="round">
                            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
                            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                        </svg>
                    </div>

                    <div style=format!(
                        "margin-top:20px;font-family:{FONT};font-size:20px;\
                         font-weight:700;color:{BRAND};letter-spacing:-0.3px;"
                    )>"Create passcode"</div>
                    <div style=format!(
                        "margin-top:8px;font-family:{FONT};font-size:14px;\
                         color:{MUTED};text-align:center;\
                         max-width:240px;line-height:1.4;"
                    )>
                        "Choose a 6-digit passcode to protect your wallet"
                    </div>

                    <PasscodeDots
                        filled=filled_set
                        error=Signal::derive(|| false)
                        shake=Signal::derive(|| false)
                    />
                </div>

                <div style="flex:1;"/>
                <Keypad on_press=on_set_press on_backspace=on_set_back/>
            </div>

            // ── Step 3: Confirm PIN ─────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::ConfirmPin { "flex" } else { "none" }
            )>
                <div style="display:flex;flex-direction:column;align-items:center;padding:32px 24px 0;">
                    <div style=format!(
                        "width:72px;height:72px;border-radius:22px;\
                         background:rgba(131,135,195,0.12);\
                         display:flex;align-items:center;justify-content:center;\
                         color:{ACCENT};"
                    )>
                        <svg width="32" height="32" viewBox="0 0 24 24" fill="none"
                            stroke="currentColor" stroke-width="1.8"
                            stroke-linecap="round" stroke-linejoin="round">
                            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
                            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                        </svg>
                    </div>

                    <div style=format!(
                        "margin-top:20px;font-family:{FONT};font-size:20px;\
                         font-weight:700;color:{BRAND};letter-spacing:-0.3px;"
                    )>"Confirm passcode"</div>
                    <div style=format!(
                        "margin-top:8px;font-family:{FONT};font-size:14px;\
                         color:{MUTED};text-align:center;\
                         max-width:240px;line-height:1.4;"
                    )>
                        {move || if error.get() {
                            "Passcodes don't match — try again"
                        } else if loading.get() {
                            "Restoring wallet…"
                        } else {
                            "Re-enter your 6-digit passcode"
                        }}
                    </div>

                    <PasscodeDots filled=filled_confirm error=error shake=shake/>
                </div>

                <div style="flex:1;"/>
                <Keypad on_press=on_confirm_press on_backspace=on_confirm_back/>
            </div>

            // ── Step 4: Success ──────────────────────────────────────────────
            <div style=move || format!(
                "flex-direction:column;flex:1;display:{};",
                if step.get() == Step::Success { "flex" } else { "none" }
            )>
                <WizardSuccess
                    title="Wallet restored"
                    subtitle="Your funds are back. Keep your recovery phrase safe — it's the only way in."
                    on_continue=Callback::new(go_home)
                />
            </div>
        </div>
    }
}
