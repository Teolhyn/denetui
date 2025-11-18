```
      _                 _         _ 
     | |               | |       (_)
   __| | ___ _ __   ___| |_ _   _ _ 
  / _` |/ _ \ '_ \ / _ \ __| | | | |
 | (_| |  __/ | | |  __/ |_| |_| | |
  \__,_|\___|_| |_|\___|\__|\__,_|_|
                                    
                                    
```

Terminal. It is like a cup of hot chocolate in the middle of a dark freezing winter evening. Or like a rocking chair next to a fireplace. That cozy place where you can truly feel one with the machine, which you don't want to leave.

But one cannot stay in this magical wonderland if they want to stay up-to-date with the daily-changing, hottest JavaScript frameworks. One day without news and Naxt.JS, Nixt.JS, or Best.JS drops. One cannot allow that to happen.

That's why I created denetui (**De**veloper **Ne**ws **TUI**. I know. I am creative :P). Denetui allows you to read most upvoted daily posts from dev.to without ever leaving your terminal.

## Installation

To use the hosted version of denetui, simply install with cargo:

```sh
cargo install denetui
```

And run:

```sh
denetui
```

## Self-host instructions

If you want to self-host the backend first

```sh
cd backend && cp .env.example .env
```

And then run the backend

```sh
cargo run --release
```

Next run

```sh
cd ../tui && cp .env.example .env
```

Copy your self-hosted server url to `.env` and then build and install

```sh
cargo build --release && cargo install --path .
```

Then you can run the TUI with

```sh
denetui
```
