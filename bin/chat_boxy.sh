#!/bin/bash

# Rolo Chat with Selective Boxy Integration
# Regular chat = efficient line rendering, System messages = fancy boxy styling

# ANSI escape codes
SAVE_CURSOR="\033[s"
RESTORE_CURSOR="\033[u"
CLEAR_SCREEN="\033[2J"
CLEAR_LINE="\033[K"
HIDE_CURSOR="\033[?25l"
SHOW_CURSOR="\033[?25h"
DISABLE_SCROLLBACK="\033[?47h"
ENABLE_SCROLLBACK="\033[?47l"

# Colors for regular chat
USER_COLOR="\033[36m"       # Cyan
ASSISTANT_COLOR="\033[32m"  # Green
DIM="\033[2m"
BOLD="\033[1m"
RESET="\033[0m"

# Layout dimensions
TERMINAL_HEIGHT=$(tput lines)
TERMINAL_WIDTH=$(tput cols)
CHAT_HEIGHT=$((TERMINAL_HEIGHT - 8))
INPUT_HEIGHT=4

# Protected zones
CHAT_CONTENT_START=3
CHAT_CONTENT_END=$((CHAT_HEIGHT - 2))
INPUT_PROMPT_ROW=$((CHAT_HEIGHT + 3))
BOTTOM_PADDING=1

# Chat state
CHAT_ORIENTATION=${CHAT_ORIENTATION:-top}
declare -a chat_messages=()
declare -a chat_types=()  # "user", "assistant", "system"
chat_scroll_offset=0
chat_inner_height=$((CHAT_CONTENT_END - CHAT_CONTENT_START + 1))

# Position cursor
goto() {
    echo -ne "\033[${1};${2}H"
}

# Create boxy system message and capture lines
create_system_boxy() {
    local message="$1"
    local theme="${2:-warning}"
    local header="${3:-⚙️ System}"

    # Generate boxy message
    echo "$message" | boxy --color "$theme" --header "$header" --width max --layout "hl"
}

# Draw basic layout (no fancy borders)
draw_layout() {
    echo -e "$DISABLE_SCROLLBACK$CLEAR_SCREEN$HIDE_CURSOR"

    # Simple title
    goto 1 1
    echo -ne "${BOLD}🚀 Rolo Chat - Selective Boxy Demo${RESET}"

    # Basic separator
    goto $((CHAT_HEIGHT + 1)) 1
    for ((i=1; i<=TERMINAL_WIDTH; i++)); do echo -n "─"; done

    # Input prompt placeholder
    goto $INPUT_PROMPT_ROW 5
    echo -n "${DIM}Type a message or /help for commands...${RESET}"
}

# Add regular chat message (fast line rendering)
add_chat_message() {
    local sender="$1"
    local message="$2"
    local type="${3:-user}"

    chat_messages+=("${sender}: ${message}")
    chat_types+=("$type")

    # Auto-scroll
    if [[ "$CHAT_ORIENTATION" == "bottom" ]]; then
        chat_scroll_offset=0
    else
        if ((${#chat_messages[@]} > chat_inner_height)); then
            chat_scroll_offset=$(( ${#chat_messages[@]} - chat_inner_height ))
        fi
    fi

    refresh_chat_area
}

# Add system message with boxy styling
add_system_message() {
    local message="$1"
    local theme="${2:-amber}"
    local header="${3:-⚙️ System}"

    # Generate the boxy message
    local boxy_output=$(create_system_boxy "$message" "$theme" "$header")

    # Find available space in chat area
    local current_row=$((CHAT_CONTENT_END - 3))  # Reserve 3 lines for boxy

    # Position and display the boxy message
    goto $current_row 1
    echo "$boxy_output"

    # Add spacer after boxy message
    goto $((current_row + 4)) 1
    echo ""
}

# Refresh regular chat area (efficient line rendering)
refresh_chat_area() {
    # Clear regular chat content (leave room for system boxes)
    for ((i=CHAT_CONTENT_START; i<=$((CHAT_CONTENT_END - 5)); i++)); do
        goto $i 2
        echo -ne "$CLEAR_LINE"
    done

    if [[ "$CHAT_ORIENTATION" == "bottom" ]]; then
        refresh_chat_bottom_up
    else
        refresh_chat_top_down
    fi
}

# Bottom-up chat rendering (regular messages only)
refresh_chat_bottom_up() {
    local total_messages=${#chat_messages[@]}
    local visible_start=$((total_messages - chat_inner_height + chat_scroll_offset))
    local visible_end=$((total_messages + chat_scroll_offset))

    # Bounds checking
    if ((visible_start < 0)); then visible_start=0; fi
    if ((visible_end > total_messages)); then visible_end=$total_messages; fi

    # Calculate positioning (leave room for system boxes)
    local effective_bottom=$((CHAT_CONTENT_END - BOTTOM_PADDING - 5))
    local message_count=$((visible_end - visible_start))
    local start_row=$((effective_bottom - message_count + 1))

    if ((start_row < CHAT_CONTENT_START)); then
        start_row=$CHAT_CONTENT_START
        message_count=$((effective_bottom - CHAT_CONTENT_START + 1))
        visible_start=$((visible_end - message_count))
    fi

    # Render regular messages only
    local row=$start_row
    for ((i=visible_start; i<visible_end && row <= effective_bottom; i++)); do
        local msg="${chat_messages[i]}"
        local type="${chat_types[i]}"

        # Skip system messages (they get boxy treatment)
        if [[ "$type" != "system" ]]; then
            goto $row 3

            # Color based on sender
            if [[ "$msg" =~ ^You: ]]; then
                echo -ne "${USER_COLOR}${msg}${RESET}"
            else
                echo -ne "${ASSISTANT_COLOR}${msg}${RESET}"
            fi
            ((row++))
        fi
    done
}

# Top-down chat rendering (regular messages only)
refresh_chat_top_down() {
    local start_msg=$chat_scroll_offset
    local end_msg=$((start_msg + chat_inner_height))

    local row=$CHAT_CONTENT_START
    for ((i=start_msg; i<end_msg && i<${#chat_messages[@]} && row <= $((CHAT_CONTENT_END - 5)); i++)); do
        if [[ $i -ge 0 ]]; then
            local msg="${chat_messages[i]}"
            local type="${chat_types[i]}"

            # Skip system messages
            if [[ "$type" != "system" ]]; then
                goto $row 3

                if [[ "$msg" =~ ^You: ]]; then
                    echo -ne "${USER_COLOR}${msg}${RESET}"
                else
                    echo -ne "${ASSISTANT_COLOR}${msg}${RESET}"
                fi
            fi
        fi
        ((row++))
    done
}

# Clear input area
clear_input() {
    goto $INPUT_PROMPT_ROW 5
    echo -ne "$CLEAR_LINE"
    goto $INPUT_PROMPT_ROW 5
    echo -ne "${BOLD}You:${RESET} "
}

# Enhanced slash commands with boxy system messages
handle_slash_command() {
    local cmd="$1"
    case "$cmd" in
        "/quit"|"/exit"|"/q")
            add_system_message "Chat session ending..." "crimson" "👋 Goodbye"
            sleep 1
            cleanup
            ;;
        "/clear"|"/cls")
            chat_messages=()
            chat_types=()
            chat_scroll_offset=0
            refresh_chat_area
            add_system_message "Chat history cleared" "green" "✅ Success"
            ;;
        "/help"|"/h")
            add_system_message "Available commands: /clear, /test, /spam, /orientation, /quit" "azure" "📖 Help"
            ;;
        "/test")
            add_system_message "Testing boxy system message integration!" "amber" "🧪 Test"
            ;;
        "/error")
            add_system_message "This is a simulated error message" "crimson" "❌ Error"
            ;;
        "/success")
            add_system_message "Operation completed successfully!" "green" "✅ Success"
            ;;
        "/spam")
            add_system_message "Generating chat spam..." "amber" "🌊 Spam Mode"
            for i in {1..10}; do
                add_chat_message "SpamBot" "Rapid fire message #$i" "assistant"
                sleep 0.1
            done
            add_system_message "Spam generation complete!" "green" "✅ Complete"
            ;;
        "/orientation")
            if [[ "$CHAT_ORIENTATION" == "bottom" ]]; then
                CHAT_ORIENTATION="top"
                add_system_message "Switched to top-down orientation" "azure" "🔄 Layout"
            else
                CHAT_ORIENTATION="bottom"
                add_system_message "Switched to bottom-up feed mode" "azure" "🔄 Layout"
            fi
            chat_scroll_offset=0
            refresh_chat_area
            ;;
        *)
            add_system_message "Unknown command: $cmd (try /help)" "crimson" "❌ Error"
            ;;
    esac
}

# Main chat loop
chat_loop() {
    draw_layout

    # Welcome with boxy system messages
    add_system_message "Welcome to Selective Boxy Chat!" "green" "🚀 Welcome"
    sleep 0.5
    add_system_message "Regular chat uses fast line rendering, system messages get fancy boxy styling!" "azure" "💡 Info"
    sleep 0.5

    # Add some regular chat
    add_chat_message "Assistant" "Hello! Try some commands to see the difference!" "assistant"
    add_chat_message "Assistant" "Regular messages are fast and efficient" "assistant"

    local input_row=$INPUT_PROMPT_ROW
    echo -e "$SHOW_CURSOR"
    goto $input_row 10

    while true; do
        goto $input_row 5
        echo -ne "${BOLD}You:${RESET} "
        read -e user_input

        if [[ -z "$user_input" ]]; then
            clear_input
            continue
        fi

        if [[ "$user_input" =~ ^/ ]]; then
            handle_slash_command "$user_input"
            clear_input
            continue
        fi

        # Regular user message - fast rendering
        add_chat_message "You" "$user_input" "user"
        clear_input

        # Simulate assistant response - also fast rendering
        sleep 0.3
        local responses=(
            "That's interesting!"
            "I like the selective boxy approach!"
            "Notice how system messages get fancy boxes?"
            "Regular chat stays fast and efficient!"
            "The visual distinction really helps!"
        )

        local response="${responses[$((RANDOM % ${#responses[@]}))]}"
        add_chat_message "Assistant" "$response" "assistant"

        goto $input_row 10
    done
}

# Cleanup
cleanup() {
    echo -e "$ENABLE_SCROLLBACK$SHOW_CURSOR$CLEAR_SCREEN"
    goto 1 1
    echo "Thanks for trying Selective Boxy Chat!"
    exit 0
}

trap cleanup INT TERM

# Main entry
main() {
    echo "🚀 Selective Boxy Chat Demo"
    echo "=========================="
    echo
    echo "Features:"
    echo "• Fast line rendering for regular chat"
    echo "• Beautiful boxy styling for system messages"
    echo "• Visual distinction between chat and system events"
    echo "• All slash commands get boxy treatment"
    echo
    echo "Try: /help, /test, /error, /success, /spam"
    echo
    read -p "Press Enter to start..."

    chat_loop
    cleanup
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi