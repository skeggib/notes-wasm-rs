FROM skeggib/rust_dev

COPY setup.sh setup.sh
RUN sh setup.sh

RUN apt update
RUN apt install -y openssh-server
RUN mkdir /run/sshd

CMD ["service", "ssh", "start", "-D"]
