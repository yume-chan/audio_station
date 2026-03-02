# AI Garbage

Play audio from multiple devices through one set of speaker.

## Client

```sh
audio_station client
```

Captures local audio, encodes with OPUS, then broadcasts on IPv4 network.

## Server

```sh
audio_station server
```

Receives IPv4 broadcast, decodes with OPUS, plays back locally.
