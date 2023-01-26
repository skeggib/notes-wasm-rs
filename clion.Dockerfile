FROM skeggib/rust_dev

RUN apt update
RUN apt install -y openssh-server
RUN mkdir /run/sshd

RUN useradd clion -rm -d /home/clion -p "$(openssl passwd -1 clion)"

CMD ["service", "ssh", "start", "-D"]
