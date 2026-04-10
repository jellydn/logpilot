#compdef logpilot

# Zsh completion for logpilot

_logpilot() {
    local curcontext="$curcontext" state line
    typeset -A opt_args

    _arguments -C \
        '(-h --help)'{-h,--help}'[Show help]' \
        '(-V --version)'{-V,--version}'[Show version]' \
        '1: :_logpilot_commands' \
        '*::arg:->args'

    case "$state" in
        args)
            case "$line[1]" in
                watch)
                    _arguments \
                        '(-p --pane)'{-p,--pane}'[Specific pane to watch]:pane:' \
                        '(-b --buffer)'{-b,--buffer}'[Buffer duration in minutes]:duration:' \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        ':session:_logpilot_sessions'
                    ;;
                summarize)
                    _arguments \
                        '(-l --last)'{-l,--last}'[Time window]:duration:' \
                        '(-f --format)'{-f,--format}'[Output format]:format:(text json)' \
                        '(-t --tokens)'{-t,--tokens}'[Max tokens]:tokens:' \
                        '--errors-only[Show only errors]' \
                        '(-h --help)'{-h,--help}'[Show help]'
                    ;;
                ask)
                    _arguments \
                        '(-c --context)'{-c,--context}'[Context duration]:duration:' \
                        '--include-logs[Include raw logs]' \
                        '(-h --help)'{-h,--help}'[Show help]' \
                        ':question:'
                    ;;
                mcp-server)
                    _arguments \
                        '(-v --verbose)'{-v,--verbose}'[Verbose logging]' \
                        '(-h --help)'{-h,--help}'[Show help]'
                    ;;
                status)
                    _arguments \
                        '(-d --detailed)'{-d,--detailed}'[Detailed output]' \
                        '(-s --session)'{-s,--session}'[Filter by session]:session:' \
                        '(-h --help)'{-h,--help}'[Show help]'
                    ;;
            esac
            ;;
    esac
}

_logpilot_commands() {
    local commands=(
        'watch:Watch a tmux session'
        'summarize:Summarize recent logs'
        'ask:Ask AI-assisted question'
        'mcp-server:Start MCP server'
        'status:Show status'
        'help:Show help'
    )
    _describe -t commands 'commands' commands "$@"
}

_logpilot_sessions() {
    local sessions
    sessions=(${(f)"$(tmux list-sessions -F "#S" 2>/dev/null)"})
    _describe -t sessions 'sessions' sessions "$@"
}

compdef _logpilot logpilot
