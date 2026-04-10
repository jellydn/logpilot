#!/bin/bash
# Bash completion for logpilot

_logpilot_completions() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    # Main commands
    local commands="watch summarize ask mcp-server status help"

    # Global options
    local global_opts="--help --version"

    case "${COMP_CWORD}" in
        1)
            COMPREPLY=( $(compgen -W "${commands}" -- ${cur}) )
            return 0
            ;;
        *)
            case "${COMP_WORDS[1]}" in
                watch)
                    local watch_opts="--pane --buffer --help"
                    if [[ ${cur} == -* ]]; then
                        COMPREPLY=( $(compgen -W "${watch_opts}" -- ${cur}) )
                    else
                        # Suggest tmux sessions
                        local sessions=$(tmux list-sessions -F "#S" 2>/dev/null)
                        COMPREPLY=( $(compgen -W "${sessions}" -- ${cur}) )
                    fi
                    return 0
                    ;;
                summarize)
                    local sum_opts="--last --format --tokens --errors-only --help"
                    COMPREPLY=( $(compgen -W "${sum_opts}" -- ${cur}) )
                    return 0
                    ;;
                ask)
                    local ask_opts="--context --include-logs --help"
                    COMPREPLY=( $(compgen -W "${ask_opts}" -- ${cur}) )
                    return 0
                    ;;
                mcp-server)
                    local mcp_opts="--verbose --help"
                    COMPREPLY=( $(compgen -W "${mcp_opts}" -- ${cur}) )
                    return 0
                    ;;
                status)
                    local status_opts="--detailed --session --help"
                    COMPREPLY=( $(compgen -W "${status_opts}" -- ${cur}) )
                    return 0
                    ;;
                help)
                    COMPREPLY=( $(compgen -W "${commands}" -- ${cur}) )
                    return 0
                    ;;
            esac
            ;;
    esac
}

complete -F _logpilot_completions logpilot
