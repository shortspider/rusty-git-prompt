echo "Building program"
cargo build --release

echo "Copying program to usr/local/bin"
sudo cp target/release/rusty_git_prompt /usr/local/bin

if ! grep -q '#rusty_git_prompt_begin' ~/.bashrc; then
    echo "Updating bashrc"
    printf "\n#rusty_git_prompt_begin" >> ~/.bashrc
    printf "\nexport CLICOLOR_FORCE=1" >> ~/.bashrc
    printf "\nPS1='%s '" '${debian_chroot:+($debian_chroot)}\[\033[01;32m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\w $(rusty_git_prompt)\[\033[00m\]\n\$' >> ~/.bashrc
    printf "\n#rusty_git_prompt_end\n" >> ~/.bashrc
else
    echo "bashrc already configured"
fi