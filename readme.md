# RS Live Cam 


A simple actix server that turns your webcamera into a online streaming service. 
It has a simple session implmennted that will check the credentials with password and username stored in the dotenv file. 

## Start project 
- create a dot env file with following vars 

```
PASSWORD=
USERNAME=

```

then cargo run for development 
``` 
cargo run 
```


for productin mode 
``` 
cargo run --release 
```

The app transmits on 0.0.0.0:8000 so it should be avaiaible on your local network.

You can change the settings of the stream in the main.rs under

```
#[derive(Debug, StructOpt)]
#[structopt(name = "mjpeg-rs")]
struct Opt {
    #[structopt(short, long, default_value = "640")]
    width: u32,

    #[structopt(short, long, default_value = "360")]
    height: u32,

    #[structopt(short, long, default_value = "30")]
    fps: u64,
}
```

Tested linux version for now, you will need the underlying openCV packages for the installation for the app to work. 
-- clang
-- clang++ 
-- libopencv-dev 
-- libclang-dev
