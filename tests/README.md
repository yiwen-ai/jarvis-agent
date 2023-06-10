## Generate Cert

https://github.com/square/certstrap

```sh
certstrap init --common-name "YiwenAI" --expires "10 year"
certstrap request-cert --common-name jarvis.agent -ip 127.0.0.1 -domain localhost
certstrap sign jarvis.agent --CA "YiwenAI"
```

```sh
openssl x509 -in yiwen.ai.crt -noout -text
```sh
