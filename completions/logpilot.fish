# Fish completion for logpilot

# Disable file completions for subcommands
complete -c logpilot -f

# Global options
complete -c logpilot -s h -l help -d "Show help"
complete -c logpilot -s V -l version -d "Show version"

# Subcommands
complete -c logpilot -n "__fish_use_subcommand" -a "watch" -d "Watch a tmux session"
complete -c logpilot -n "__fish_use_subcommand" -a "summarize" -d "Summarize recent logs"
complete -c logpilot -n "__fish_use_subcommand" -a "ask" -d "Ask AI-assisted question"
complete -c logpilot -n "__fish_use_subcommand" -a "mcp-server" -d "Start MCP server"
complete -c logpilot -n "__fish_use_subcommand" -a "status" -d "Show status"
complete -c logpilot -n "__fish_use_subcommand" -a "help" -d "Show help"

# Watch command options
complete -c logpilot -n "__fish_seen_subcommand_from watch" -s p -l pane -d "Specific pane to watch"
complete -c logpilot -n "__fish_seen_subcommand_from watch" -s b -l buffer -d "Buffer duration in minutes"
complete -c logpilot -n "__fish_seen_subcommand_from watch" -s h -l help -d "Show help"

# Watch command - tmux sessions
complete -c logpilot -n "__fish_seen_subcommand_from watch; and not __fish_prev_arg_in -p --pane -b --buffer -h --help" -a "(tmux list-sessions -F '#S' 2>/dev/null)" -d "tmux session"

# Summarize command options
complete -c logpilot -n "__fish_seen_subcommand_from summarize" -s l -l last -d "Time window (e.g., 10m, 1h)"
complete -c logpilot -n "__fish_seen_subcommand_from summarize" -s f -l format -d "Output format" -a "text json"
complete -c logpilot -n "__fish_seen_subcommand_from summarize" -s t -l tokens -d "Max tokens"
complete -c logpilot -n "__fish_seen_subcommand_from summarize" -l errors-only -d "Show only errors"
complete -c logpilot -n "__fish_seen_subcommand_from summarize" -s h -l help -d "Show help"

# Ask command options
complete -c logpilot -n "__fish_seen_subcommand_from ask" -s c -l context -d "Context duration"
complete -c logpilot -n "__fish_seen_subcommand_from ask" -l include-logs -d "Include raw logs"
complete -c logpilot -n "__fish_seen_subcommand_from ask" -s h -l help -d "Show help"

# MCP server options
complete -c logpilot -n "__fish_seen_subcommand_from mcp-server" -s v -l verbose -d "Verbose logging"
complete -c logpilot -n "__fish_seen_subcommand_from mcp-server" -s h -l help -d "Show help"

# Status options
complete -c logpilot -n "__fish_seen_subcommand_from status" -s d -l detailed -d "Detailed output"
complete -c logpilot -n "__fish_seen_subcommand_from status" -s s -l session -d "Filter by session"
complete -c logpilot -n "__fish_seen_subcommand_from status" -s h -l help -d "Show help"
