# Bash Completions

Complete setup guide for Bash shell completions.

## Installation

### Linux

```bash
# Most distros - standard location
blz completions bash | sudo tee /usr/share/bash-completion/completions/blz

# User-specific (no sudo required)
mkdir -p ~/.local/share/bash-completion/completions
blz completions bash > ~/.local/share/bash-completion/completions/blz

# Reload
source ~/.bashrc
```

### macOS

```bash
# Install bash-completion first
brew install bash-completion@2

# Add to ~/.bash_profile
echo '[[ -r "$(brew --prefix)/etc/profile.d/bash_completion.sh" ]] && . "$(brew --prefix)/etc/profile.d/bash_completion.sh"' >> ~/.bash_profile

# Install completions
blz completions bash > $(brew --prefix)/etc/bash_completion.d/blz

# Reload
source ~/.bash_profile
```

## Features

- Command completion
- Option/flag completion
- Filename completion for paths

## Testing

```bash
blz <TAB><TAB>           # List commands
blz search --<TAB><TAB>  # List options
blz add <TAB><TAB>       # Complete filenames
```

## Troubleshooting

### Completions not working

```bash
# Check bash-completion is loaded
type _init_completion

# If not found, ensure bash-completion is installed
# Then source it manually
source /usr/share/bash-completion/bash_completion
```

### Old Bash version

Bash 3.x (macOS default) has limited completion support. Consider:

```bash
# Install newer Bash
brew install bash

# Add to /etc/shells
echo $(brew --prefix)/bin/bash | sudo tee -a /etc/shells

# Change default shell
chsh -s $(brew --prefix)/bin/bash
```