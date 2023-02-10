## Example usage

Set-up:
```
mkdir -p ~/workspace/server
mkdir -p ~/workspace/cient
```

Run server and client in separate terminals:
```
cargo run ~/workspace/server server
cargo run ~/workspace/client client
```

In another terminal:
```
> cat ~/workspace/server/note_1.txt
Example note 1

Some text


> cat ~/workspace/client/note_1.txt
Example note 1

Some text


> echo "Hello world" >> ~/workspace/server/note_1.txt


> cat ~/workspace/server/note_1.txt
Example note 1

Some text
Hello world


> cat ~/workspace/client/note_1.txt
Example note 1

Some text
Hello world
```
