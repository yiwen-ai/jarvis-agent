# Jarvis Agent

## 生成 Linux 执行文件

```sh
make build
```
生成文件位于 ./target/app/jarvis-agent

/home/yiwen/jarvis

## 复制到远程服务器

```sh
scp target/app/jarvis-agent yiwen@74.235.107.193:/home/yiwen/jarvis/
```

准备好配置文件和证书文件，执行：
```sh
CONFIG_FILE_PATH=./config.toml nohup ./jarvis-agent > jarvis.out 2>&1 &
```