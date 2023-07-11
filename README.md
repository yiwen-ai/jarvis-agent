# Jarvis Agent

## 生成 Linux 执行文件

```sh
make build
```
生成文件位于 ./target/app/jarvis-agent

/home/yiwen/jarvis

## 复制到远程服务器

```sh
scp target/app/jarvis-agent yiwen@74.235.107.193:/home/yiwen/jarvis/jarvis-agent-next
```

准备好配置文件和证书文件，执行：
```sh
ps -ax | grep jarvis-agent
kill xxx
mv jarvis-agent jarvis-agent-bk
mv jarvis-agent-next jarvis-agent

CONFIG_FILE_PATH=./config.toml nohup ./jarvis-agent > jarvis.log 2>&1 &
```