#!/bin/bash

# Ultimate Chat Facts Paradise - Selective Boxy + Facts + Flicker-Free Multi-Pane!

# Terminal setup
clear
printf "\033[?1049h"  # Enter alternate screen
printf "\033[?25l"    # Hide cursor
stty -echo

# Terminal dimensions
TERM_WIDTH=$(tput cols)
TERM_HEIGHT=$(tput lines)

# Colors for regular chat
USER_COLOR="\033[36m"       # Cyan
ASSISTANT_COLOR="\033[32m"  # Green
DIM="\033[2m"
BOLD="\033[1m"
RESET="\033[0m"

# MEGA FACTS DATABASE! 🎉
RANDOM_FACTS=(
    "🌊 The wave emoji looks like a gremlin from far away! 😂"
    "🐙 Octopuses have three hearts and blue blood!"
    "🍯 Honey never spoils - archaeologists found edible honey in Egyptian tombs!"
    "🦋 Butterflies taste with their feet!"
    "🌙 A day on Venus is longer than its year!"
    "🐧 Penguins have knees hidden inside their bodies!"
    "🌈 Bananas are berries, but strawberries aren't!"
    "🧠 Your brain uses 20% of your body's energy despite being 2% of your weight!"
    "🐠 Goldfish can live for over 40 years with proper care!"
    "⚡ Lightning strikes the Earth 8 million times per day!"
    "🌍 Earth is the only known planet where fire can naturally occur!"
    "🦎 Geckos can run up glass surfaces due to van der Waals forces!"
    "📡 Radio waves from Earth have traveled about 100 light-years into space!"
    "🎵 The longest recorded flight of a chicken is 13 seconds!"
    "🌕 The Moon is moving away from Earth at 3.8cm per year!"
    "🐜 Ants can lift 10-50 times their own body weight!"
    "🎨 Shrimp can see 16 types of color receptors (humans only see 3)!"
    "🌋 There are more possible chess games than atoms in the observable universe!"
    "🦆 Ducks have waterproof feathers that never get wet!"
    "🌪️ A cloud can weigh more than a million pounds!"
    "🦘 Kangaroos can't walk backwards!"
    "🐨 Koalas sleep 18-22 hours per day!"
    "🎯 Your stomach gets an entirely new lining every 3-5 days!"
    "🌊 There's more water in the atmosphere than in all rivers combined!"
    "🦖 T-Rex lived closer in time to humans than to Stegosaurus!"
    "🍄 The largest organism on Earth is a fungus in Oregon!"
    "⭐ Neutron stars are so dense that a teaspoon would weigh 6 billion tons!"
    "🐙 Octopuses can change color faster than a chameleon!"
    "🌱 Bamboo can grow up to 3 feet in a single day!"
    "🦅 Peregrine falcons can dive at speeds over 240 mph!"
    "🧬 You share 50% of your DNA with bananas!"
    "🌊 The pressure at the bottom of the ocean would crush you instantly!"
    "🎪 A group of flamingos is called a 'flamboyance'!"
    "🚀 Space is completely silent because there's no air to carry sound!"
    "🦈 Sharks have been around longer than trees!"
    "🌸 Cherry blossoms are actually related to roses!"
    "🧊 Hot water can freeze faster than cold water (Mpemba effect)!"
    "🐧 Emperor penguins can hold their breath for 20+ minutes!"
    "🌈 There are infinite shades of green your eyes can distinguish!"
    "⚡ Your body produces enough heat in 30 minutes to boil water!"
)

# Chat buffers and state
declare -a chat_messages=()
declare -a chat_types=()  # "user", "assistant", "system", "fact"
declare -a facts_buffer=()
FACTS_SIZE=3
FACTS_SHOWN=0

# System content
SYSTEM_CONTENT="CPU: 67% ██████▓░░░
RAM: 84% ████████▓░
Facts learned: 0
Messages: 0"

CODE_CONTENT="$ rolo --ultimate-chat enabled
  ✓ Selective boxy: ACTIVE
  ✓ Facts paradise: LOADED
  ✓ Flicker-free: GUARANTEED
$ chat --features
  💬 Fast regular chat
  🎲 Auto-facts every 8s
  📦 Boxy system messages"

# Content tracking for flicker-free updates
PREV_CHAT_CONTENT=""
PREV_FACTS_CONTENT=""
PREV_SYSTEM_CONTENT=""
PREV_CODE_CONTENT=""

# ANSI positioning function
goto() {
    printf "\033[%d;%dH" "$1" "$2"
}

# Clear specific area
clear_area() {
    local start_row=$1
    local start_col=$2
    local width=$3
    local height=$4

    for ((row=start_row; row<start_row+height; row++)); do
        goto $row $start_col
        printf "%*s" $width ""
    done
}

# Create boxy system message
create_system_boxy() {
    local message="$1"
    local theme="${2:-warning}"
    local header="${3:-⚙️ System}"

    echo "$message" | boxy --color "$theme" --header "$header" --width 38
}

# Add regular chat message (fast line rendering in chat pane)
add_chat_message() {
    local sender="$1"
    local message="$2"
    local type="${3:-user}"

    chat_messages+=("${sender}: ${message}")
    chat_types+=("$type")

    # Keep only last 6 messages for chat pane
    if [ ${#chat_messages[@]} -gt 6 ]; then
        chat_messages=("${chat_messages[@]:1}")
        chat_types=("${chat_types[@]:1}")
    fi
}

# Add system message (gets boxy treatment)
add_system_message() {
    local message="$1"
    local theme="${2:-amber}"
    local header="${3:-⚙️ System}"

    # Add to chat buffer with system type
    chat_messages+=("SYSTEM: $message")
    chat_types+=("system")

    # Keep buffer size
    if [ ${#chat_messages[@]} -gt 6 ]; then
        chat_messages=("${chat_messages[@]:1}")
        chat_types=("${chat_types[@]:1}")
    fi
}

# Add random fact to facts buffer
add_random_fact() {
    local new_fact="${RANDOM_FACTS[$((RANDOM % ${#RANDOM_FACTS[@]}))]}"
    facts_buffer=("$new_fact" "${facts_buffer[@]}")
    if [ ${#facts_buffer[@]} -gt $FACTS_SIZE ]; then
        facts_buffer=("${facts_buffer[@]:0:$FACTS_SIZE}")
    fi
    ((FACTS_SHOWN++))
}

# Render boxy content at specific positions ONLY if changed
render_boxy_if_changed() {
    local content="$1"
    local prev_content="$2"
    local start_row=$3
    local start_col=$4
    local max_width=$5
    local max_height=$6

    if [[ "$content" != "$prev_content" ]]; then
        clear_area $start_row $start_col $max_width $max_height
        local line_num=0
        while IFS= read -r line; do
            goto $((start_row + line_num)) $start_col
            printf "%s" "$line"
            ((line_num++))
        done <<< "$content"
    fi
}

# Generate chat pane content with selective boxy rendering
generate_chat_content() {
    local chat_content=""

    for i in "${!chat_messages[@]}"; do
        local msg="${chat_messages[i]}"
        local type="${chat_types[i]}"

        if [[ "$type" == "system" ]]; then
            # Extract the system message and create boxy version
            local system_msg="${msg#SYSTEM: }"
            local boxy_msg=$(echo "$system_msg" | boxy --color amber --header "⚙️ System" --width 36)
            chat_content+="$boxy_msg"$'\n'
        else
            # Regular message with fast line rendering
            if [[ "$msg" =~ ^You: ]]; then
                chat_content+="${USER_COLOR}${msg}${RESET}"$'\n'
            else
                chat_content+="${ASSISTANT_COLOR}${msg}${RESET}"$'\n'
            fi
        fi
    done

    echo -n "$chat_content"
}

# Main render function - FLICKER-FREE!
render_all_panes() {
    # Generate current content
    local current_chat_content=$(generate_chat_content)

    local facts_content=""
    for fact in "${facts_buffer[@]}"; do
        facts_content+="$fact"$'\n'
    done

    local current_chat_boxy=$(echo -n "$current_chat_content" | boxy --style rounded --color azure --header "💬 Ultimate Chat" --width 40)
    local current_facts_boxy=$(echo -n "$facts_content" | boxy --style double --color magenta --header "🎲 Auto Facts!" --width 45)
    local current_system_boxy=$(echo "$SYSTEM_CONTENT" | boxy --style heavy --color green --header "📊 Chat Stats" --width 30)
    local current_code_boxy=$(echo "$CODE_CONTENT" | boxy --style ascii --color yellow --header "⚡ Ultimate Control" --width 35)

    # Status bar (only on first render)
    if [[ -z "$PREV_CHAT_CONTENT" ]]; then
        goto 1 1
        printf "\033[2K\033[7m%*s\033[0m" $TERM_WIDTH " ROLO Ultimate Chat Paradise - Selective Boxy + Auto Facts + Zero Flicker! Press 'q' to quit "
    fi

    # Render panes ONLY if changed (Zellij pattern!)
    render_boxy_if_changed "$current_chat_boxy" "$PREV_CHAT_CONTENT" 3 2 42 12      # Chat with selective boxy
    render_boxy_if_changed "$current_facts_boxy" "$PREV_FACTS_CONTENT" 3 45 47 10    # Auto facts
    render_boxy_if_changed "$current_system_boxy" "$PREV_SYSTEM_CONTENT" 16 2 32 8   # Stats
    render_boxy_if_changed "$current_code_boxy" "$PREV_CODE_CONTENT" 16 36 37 8      # Control

    # Update previous content
    PREV_CHAT_CONTENT="$current_chat_boxy"
    PREV_FACTS_CONTENT="$current_facts_boxy"
    PREV_SYSTEM_CONTENT="$current_system_boxy"
    PREV_CODE_CONTENT="$current_code_boxy"

    # Update facts counter in status
    goto 1 75
    printf "Facts: $FACTS_SHOWN"
}

# Update functions
update_system_stats() {
    local cpu=$((RANDOM % 100))
    local ram=$((RANDOM % 100))

    SYSTEM_CONTENT="CPU: ${cpu}% $(printf '█%.0s' $(seq 1 $((cpu/10))))$(printf '░%.0s' $(seq 1 $((10-cpu/10))))
RAM: ${ram}% $(printf '█%.0s' $(seq 1 $((ram/10))))$(printf '░%.0s' $(seq 1 $((10-ram/10))))
Facts learned: $FACTS_SHOWN
Messages: ${#chat_messages[@]}"
}

update_code_control() {
    local timestamp=$(date +%H:%M:%S)

    CODE_CONTENT="$ rolo --ultimate-chat enabled
  ✓ Selective boxy: ACTIVE
  ✓ Facts paradise: LOADED
  ✓ Flicker-free: GUARANTEED
$ chat --status
  💬 Messages: ${#chat_messages[@]}
  🎲 Facts: $FACTS_SHOWN
  ⏰ Updated: $timestamp"
}

# Handle slash commands with boxy system messages
handle_slash_command() {
    local cmd="$1"
    case "$cmd" in
        "/quit"|"/exit"|"/q")
            add_system_message "Thanks for chatting! You learned $FACTS_SHOWN facts!" "crimson" "👋 Goodbye"
            return 1
            ;;
        "/help"|"/h")
            add_system_message "Commands: /fact, /clear, /spam, /quit. Facts auto-appear every 8s!" "azure" "📖 Help"
            ;;
        "/fact")
            add_random_fact
            add_system_message "Random fact added to facts pane!" "green" "🎲 Fact"
            ;;
        "/clear")
            chat_messages=()
            chat_types=()
            add_system_message "Chat cleared! Facts continue auto-streaming." "green" "✅ Clear"
            ;;
        "/spam")
            add_system_message "Generating chat spam..." "amber" "🌊 Spam"
            for i in {1..5}; do
                add_chat_message "SpamBot" "Rapid message #$i about amazing facts!" "assistant"
            done
            add_system_message "Spam complete! Notice selective boxy system messages!" "green" "✅ Complete"
            ;;
        *)
            add_system_message "Unknown command: $cmd (try /help)" "crimson" "❌ Error"
            ;;
    esac
    return 0
}

# Interactive input handling
handle_input() {
    local input_row=$((TERM_HEIGHT - 2))
    goto $input_row 1
    printf "\033[2K${BOLD}You:${RESET} "

    # Simple input reading
    local user_input=""
    local char=""

    while true; do
        read -n 1 -s char

        if [[ "$char" == $'\n' || "$char" == $'\r' ]]; then
            break
        elif [[ "$char" == $'\177' || "$char" == $'\b' ]]; then
            # Backspace
            if [[ ${#user_input} -gt 0 ]]; then
                user_input="${user_input%?}"
                printf "\b \b"
            fi
        elif [[ "$char" == $'\e' ]]; then
            # Escape sequence - ignore for now
            read -n 2 -s
        else
            user_input+="$char"
            printf "%s" "$char"
        fi
    done

    # Clear input line
    goto $input_row 1
    printf "\033[2K"

    if [[ -n "$user_input" ]]; then
        if [[ "$user_input" =~ ^/ ]]; then
            if ! handle_slash_command "$user_input"; then
                return 1  # Quit command
            fi
        else
            # Regular user message
            add_chat_message "You" "$user_input" "user"

            # Simulate assistant response
            local responses=(
                "That's fascinating! Did you know about the facts streaming in?"
                "I love the selective boxy system messages!"
                "Notice how regular chat is fast and facts auto-appear?"
                "The flicker-free updates make this so smooth!"
                "System messages get beautiful boxy styling!"
                "Facts keep coming every 8 seconds automatically!"
            )
            local response="${responses[$((RANDOM % ${#responses[@]}))]}"
            add_chat_message "Assistant" "$response" "assistant"
        fi
    fi

    return 0
}

# Main demo loop
main() {
    echo "🚀 ROLO Ultimate Chat Facts Paradise!"
    echo "Combining selective boxy + auto facts + flicker-free rendering..."

    # Initialize with some content
    add_chat_message "Assistant" "Welcome to Ultimate Chat Paradise!" "assistant"
    add_chat_message "Assistant" "Facts auto-stream, system msgs get boxy styling!" "assistant"
    add_random_fact
    add_system_message "Ultimate Chat Paradise initialized!" "green" "🚀 Welcome"

    echo "Press any key to start..."
    read -n 1 -s

    # Initial render
    printf "\033[2J"
    render_all_panes

    local last_fact_update=0
    local last_stats_update=0

    # Main loop with background updates
    while true; do
        local current_time=$SECONDS

        # Auto-add facts every 8 seconds
        if [ $((current_time - last_fact_update)) -ge 8 ]; then
            add_random_fact
            last_fact_update=$current_time
        fi

        # Update stats every 5 seconds
        if [ $((current_time - last_stats_update)) -ge 5 ]; then
            update_system_stats
            update_code_control
            last_stats_update=$current_time
        fi

        # Render changes
        render_all_panes

        # Handle input with timeout
        if read -t 0.3 -n 1 key 2>/dev/null; then
            if [[ "$key" == "q" ]]; then
                break
            else
                # Put the character back and handle full input
                printf "%s" "$key"
                if ! handle_input; then
                    break
                fi
            fi
        fi

        sleep 0.1
    done
}

# Cleanup
cleanup() {
    printf "\033[?25h"    # Show cursor
    printf "\033[?1049l"  # Exit alternate screen
    stty echo
    clear
    echo "✨ Ultimate Chat Facts Paradise complete!"
    echo "🎲 You learned $FACTS_SHOWN amazing facts!"
    echo "💬 Experienced selective boxy + flicker-free perfection!"
    echo "🚀 This is the future of terminal chat interfaces!"
}

trap cleanup EXIT
main