use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use rustok_types::WalletInfo;
use serde::Serialize;

use crate::app::WalletState;
use crate::bridge::tauri_invoke;

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
struct ImportArgs {
    phrase: String,
    password: String,
}

/// Four-step wizard for creating a new wallet with BIP39 recovery phrase.
///
/// 1. Backup intro — three acknowledgement checkboxes.
/// 2. Show 12-word phrase — user writes it down.
/// 3. Confirm quiz — pick three words from the phrase to prove the user
///    actually wrote it down.
/// 4. Password — derive wallet and persist via `import_wallet_from_mnemonic`.
#[component]
pub fn WalletPage() -> impl IntoView {
    let auth_state = use_context::<RwSignal<WalletState>>()
        .expect("WalletState context missing — must be provided in App");
    let navigate = use_navigate();

    let (step, set_step) = signal(1u8);
    let (ack_seen, set_ack_seen) = signal([false, false, false]);
    let (phrase, set_phrase) = signal(None::<String>);
    let (saved_check, set_saved_check) = signal(false);
    // quiz state: 3 indices into the phrase, 4 options each, user's pick per question.
    let (quiz_indices, set_quiz_indices) = signal(Vec::<usize>::new());
    let (quiz_options, set_quiz_options) = signal(Vec::<Vec<String>>::new());
    let (quiz_selected, set_quiz_selected) = signal(Vec::<Option<String>>::new());
    let (password, set_password) = signal(String::new());
    let (confirm, set_confirm) = signal(String::new());
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);

    let go_to_phrase = move |_| {
        set_error.set(None);
        set_loading.set(true);
        spawn_local(async move {
            match tauri_invoke::<_, String>("generate_mnemonic_phrase", &EmptyArgs {}).await {
                Ok(p) => {
                    set_phrase.set(Some(p));
                    set_step.set(2);
                }
                Err(e) => set_error.set(Some(e)),
            }
            set_loading.set(false);
        });
    };

    let go_to_quiz = move |_| {
        set_error.set(None);
        let Some(p) = phrase.get() else { return };
        let (indices, options) = build_quiz(&p);
        set_quiz_indices.set(indices);
        set_quiz_options.set(options);
        set_quiz_selected.set(vec![None; 3]);
        set_step.set(3);
    };

    let go_to_password = move |_| {
        set_error.set(None);
        set_step.set(4);
    };

    let create_wallet = {
        let navigate = navigate.clone();
        move |_| {
            let pwd = password.get();
            let pwd_confirm = confirm.get();
            let ph = phrase.get().unwrap_or_default();

            if pwd.len() < 8 {
                set_error.set(Some("Password must be at least 8 characters".into()));
                return;
            }
            if pwd != pwd_confirm {
                set_error.set(Some("Passwords do not match".into()));
                return;
            }
            if ph.is_empty() {
                set_error.set(Some("Phrase missing — restart wizard".into()));
                return;
            }

            set_loading.set(true);
            set_error.set(None);

            let navigate = navigate.clone();
            spawn_local(async move {
                match tauri_invoke::<_, WalletInfo>(
                    "import_wallet_from_mnemonic",
                    &ImportArgs {
                        phrase: ph,
                        password: pwd,
                    },
                )
                .await
                {
                    Ok(_) => {
                        auth_state.set(WalletState::Unlocked);
                        navigate("/", Default::default());
                    }
                    Err(e) => set_error.set(Some(e)),
                }
                set_loading.set(false);
            });
        }
    };

    let back = move |_| {
        set_error.set(None);
        let s = step.get();
        if s > 1 {
            set_step.set(s - 1);
        }
    };

    // Quiz completion: all three picks match the corresponding phrase word.
    let quiz_complete = move || {
        let Some(p) = phrase.get() else { return false };
        let words: Vec<String> = p.split_whitespace().map(String::from).collect();
        let idxs = quiz_indices.get();
        let selected = quiz_selected.get();
        if selected.len() != idxs.len() {
            return false;
        }
        idxs.iter().zip(selected.iter()).all(|(i, sel)| {
            sel.as_ref().is_some_and(|s| words.get(*i) == Some(s))
        })
    };

    view! {
        <div class="wallet-create">
            <div class="unlock-title">"Create Wallet"</div>

            {move || error.get().map(|e| view! {
                <p class="text-red-400 mt-2 text-center">{e}</p>
            })}

            // Step 1 — backup intro
            <div style:display=move || if step.get() == 1 { "" } else { "none" }>
                <p class="text-gray-300 mb-4 text-center">
                    "Before you get your recovery phrase, acknowledge each item:"
                </p>
                {[
                    "My recovery phrase is the ONLY way to restore this wallet. If I lose it, my funds are gone.",
                    "I will write the 12 words down on paper and store them somewhere safe — never in a screenshot, cloud, or chat.",
                    "Anyone who sees these words can steal all my funds. I will never share them — not even with support.",
                ].iter().enumerate().map(|(i, text)| {
                    let text = text.to_string();
                    view! {
                        <label class="ack-row">
                            <input
                                type="checkbox"
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    set_ack_seen.update(|arr| arr[i] = checked);
                                }
                            />
                            <span>{text}</span>
                        </label>
                    }
                }).collect_view()}

                <button
                    class="bg-indigo-600 px-4 py-3 rounded-xl w-full hover:bg-indigo-700 mt-4 disabled:bg-gray-700"
                    on:click=go_to_phrase
                    disabled=move || !ack_seen.get().iter().all(|b| *b) || loading.get()
                >
                    {move || if loading.get() { "Generating..." } else { "Show Recovery Phrase" }}
                </button>
            </div>

            // Step 2 — show 12 words
            <div style:display=move || if step.get() == 2 { "" } else { "none" }>
                <p class="text-gray-300 mb-3 text-center">
                    "Write these 12 words down in order."
                </p>
                <p class="text-yellow-400 text-sm mb-4 text-center">
                    "Never share. Never paste anywhere online."
                </p>

                <div class="mnemonic-grid">
                    {move || phrase.get().map(|p| {
                        p.split_whitespace()
                            .enumerate()
                            .map(|(i, word)| view! {
                                <div class="mnemonic-word">
                                    <span class="mnemonic-index">{i + 1}.</span>
                                    <span class="mnemonic-text">{word.to_string()}</span>
                                </div>
                            })
                            .collect_view()
                    })}
                </div>

                <label class="ack-row mt-4">
                    <input
                        type="checkbox"
                        on:change=move |ev| set_saved_check.set(event_target_checked(&ev))
                    />
                    <span>"I have written down all 12 words in order."</span>
                </label>

                <button
                    class="bg-indigo-600 px-4 py-3 rounded-xl w-full hover:bg-indigo-700 mt-4 disabled:bg-gray-700"
                    on:click=go_to_quiz
                    disabled=move || !saved_check.get()
                >
                    "Continue"
                </button>
                <button
                    class="text-gray-400 text-sm w-full text-center mt-2"
                    on:click=back
                >
                    "← Back"
                </button>
            </div>

            // Step 3 — confirm quiz
            <div style:display=move || if step.get() == 3 { "" } else { "none" }>
                <p class="text-gray-300 mb-4 text-center">
                    "Let's make sure you wrote down the words. Pick the word at each position:"
                </p>

                {move || {
                    let idxs = quiz_indices.get();
                    let options = quiz_options.get();
                    let selected = quiz_selected.get();
                    let words: Vec<String> = phrase.get()
                        .unwrap_or_default()
                        .split_whitespace()
                        .map(String::from)
                        .collect();

                    idxs.into_iter().enumerate().map(|(q, position)| {
                        let opts = options.get(q).cloned().unwrap_or_default();
                        let user_pick = selected.get(q).cloned().flatten();
                        let correct_word = words.get(position).cloned().unwrap_or_default();

                        view! {
                            <div class="quiz-question">
                                <div class="quiz-label">
                                    {format!("Word #{}", position + 1)}
                                </div>
                                <div class="quiz-options">
                                    {opts.into_iter().map(|opt| {
                                        let user_pick = user_pick.clone();
                                        let correct_word = correct_word.clone();
                                        let opt_label = opt.clone();
                                        let opt_for_click = opt.clone();
                                        let class = move || {
                                            match &user_pick {
                                                Some(picked) if picked == &opt => {
                                                    if picked == &correct_word {
                                                        "quiz-option correct"
                                                    } else {
                                                        "quiz-option wrong"
                                                    }
                                                }
                                                _ => "quiz-option",
                                            }
                                        };
                                        view! {
                                            <button
                                                class=class
                                                on:click=move |_| {
                                                    let pick = opt_for_click.clone();
                                                    set_quiz_selected.update(|s| {
                                                        if let Some(slot) = s.get_mut(q) {
                                                            *slot = Some(pick);
                                                        }
                                                    });
                                                }
                                            >
                                                {opt_label}
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        }
                    }).collect_view()
                }}

                <button
                    class="bg-indigo-600 px-4 py-3 rounded-xl w-full hover:bg-indigo-700 mt-4 disabled:bg-gray-700"
                    on:click=go_to_password
                    disabled=move || !quiz_complete()
                >
                    "Continue"
                </button>
                <button
                    class="text-gray-400 text-sm w-full text-center mt-2"
                    on:click=back
                >
                    "← Back"
                </button>
            </div>

            // Step 4 — password
            <div style:display=move || if step.get() == 4 { "" } else { "none" }>
                <p class="text-gray-300 mb-4 text-center">
                    "Set a password to unlock this wallet on this device."
                </p>
                <input
                    type="password"
                    class="border border-gray-600 rounded-xl p-2 w-full bg-gray-800 text-white mb-2"
                    placeholder="Password (min 8 characters)"
                    on:input:target=move |ev| set_password.set(ev.target().value())
                />
                <input
                    type="password"
                    class="border border-gray-600 rounded-xl p-2 w-full bg-gray-800 text-white"
                    placeholder="Confirm password"
                    on:input:target=move |ev| set_confirm.set(ev.target().value())
                />
                <button
                    class="mt-4 bg-indigo-600 px-4 py-3 rounded-xl w-full hover:bg-indigo-700 disabled:bg-gray-700"
                    on:click=create_wallet
                    disabled=move || loading.get()
                >
                    {move || if loading.get() { "Creating..." } else { "Create Wallet" }}
                </button>
                <button
                    class="text-gray-400 text-sm w-full text-center mt-2"
                    on:click=back
                >
                    "← Back"
                </button>
            </div>

            // Footer links — existing wallet or restore.
            <p class="text-gray-400 text-sm mt-6 text-center">
                "Have a recovery phrase? "
                <a href="/wallet/restore" class="text-blue-400">"Restore wallet"</a>
            </p>
        </div>
    }
}

fn event_target_checked(ev: &leptos::ev::Event) -> bool {
    use web_sys::wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|el| el.checked())
        .unwrap_or(false)
}

/// Build quiz data: three sorted random indices from the phrase, plus a
/// four-option list for each — correct word + three random distractors
/// from the other phrase words, all shuffled.
fn build_quiz(phrase: &str) -> (Vec<usize>, Vec<Vec<String>>) {
    let words: Vec<String> = phrase.split_whitespace().map(String::from).collect();
    let n = words.len();
    if n < 4 {
        return (Vec::new(), Vec::new());
    }

    // 3 unique random indices, sorted ascending for display.
    let mut all: Vec<usize> = (0..n).collect();
    shuffle(&mut all);
    let mut indices: Vec<usize> = all.into_iter().take(3).collect();
    indices.sort_unstable();

    let options: Vec<Vec<String>> = indices
        .iter()
        .map(|&i| {
            let correct = words[i].clone();
            // Pool = other words, dedup to avoid repeats of the correct word.
            let mut pool: Vec<String> = words
                .iter()
                .filter(|w| **w != correct)
                .cloned()
                .collect();
            shuffle(&mut pool);
            pool.truncate(3);
            pool.push(correct);
            shuffle(&mut pool);
            pool
        })
        .collect();

    (indices, options)
}

/// Fisher–Yates shuffle using the WebCrypto/JS RNG via `js_sys::Math::random`.
fn shuffle<T>(v: &mut [T]) {
    let n = v.len();
    for i in (1..n).rev() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let j = (js_sys::Math::random() * ((i + 1) as f64)).floor() as usize;
        v.swap(i, j);
    }
}
