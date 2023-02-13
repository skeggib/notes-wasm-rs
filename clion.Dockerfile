FROM ubuntu:latest


ENV TZ=Europe/Paris
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

# install tools
RUN apt update
RUN apt install -y bash-completion
RUN apt install -y curl
RUN apt install -y gcc
RUN apt install -y git
RUN apt install -y htop
RUN apt install -y less
RUN apt install -y libssl-dev
RUN apt install -y pkg-config
RUN apt install -y tmux
RUN apt install -y tree
RUN apt install -y vim

# get and install dotfiles
WORKDIR /root
RUN git clone https://github.com/skeggib/dotfiles.git
RUN rm -f .bashrc .inputrc .vimrc .gdbinit .gitconfig .tmux.conf .tmux.conf.local
RUN ln -s dotfiles/.bashrc .bashrc
RUN ln -s dotfiles/.gdbinit .gdbinit
RUN ln -s dotfiles/.gitconfig .gitconfig
RUN ln -s dotfiles/.inputrc .inputrc
RUN ln -s dotfiles/.tmux.conf .tmux.conf
RUN ln -s dotfiles/.tmux.conf.local .tmux.conf.local
RUN ln -s dotfiles/.vimrc .vimrc

# install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

RUN apt install -y make
RUN $HOME/.cargo/bin/cargo install wasm-pack
RUN $HOME/.cargo/bin/cargo install trunk
RUN $HOME/.cargo/bin/cargo install cargo-make

RUN $HOME/.cargo/bin/rustup target add wasm32-unknown-unknown

RUN apt update
RUN apt install -y openssh-server
RUN mkdir /run/sshd

CMD ["service", "ssh", "start", "-D"]
