use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;


pub fn nearby(file: &str) -> io::Result<PathBuf> {
    use std::env;
    let current_exe = env::current_exe()?;
    let path = current_exe.parent().unwrap().join(file);
    std::fs::create_dir_all(path.parent().unwrap())?;
    Ok(path)
}

pub struct ConfigFile<C: Serialize> {
    path: PathBuf,
    modified: bool,
    data: C,
}

impl<C: Serialize + DeserializeOwned + Default> ConfigFile<C> {
    
    pub fn new<P: AsRef<Path>>(path: P) -> Result<(Self,bool), io::Error> {
        let path = path.as_ref();
        let (data, created) = Self::load(path)?;
        Ok(
            (Self { path: path.to_path_buf(), modified: false, data }, created)
        )
    }

    pub fn new_nearby(file: &str) -> Result<(Self,bool), io::Error> {
        let path = nearby(file)?;
        let (data, created) = Self::load(path.as_path())?;
        Ok(
            (Self { path, modified: false, data }, created)
        )
    }

    fn load<P: AsRef<Path>>(file: P) -> Result<(C, bool), io::Error> {
        match File::open(file.as_ref()) {
            Ok(ifile) => serde_json::from_reader(ifile).map(|data| (data, false)).map_err(io::Error::from),
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    let config: C = Default::default();
                    let ofile = File::create(file.as_ref())?;
                    serde_json::to_writer_pretty(ofile, &config).map_err(io::Error::from)?;
                    Ok((config, true))
                } else {
                    Err(e)
                }
            }
        }
    }
}

impl<C: Serialize> ConfigFile<C> {
    
    pub fn save(&mut self) -> io::Result<()> {
        let ofile = File::create(self.path.as_path())?;
        serde_json::to_writer_pretty(ofile, &self.data).map_err(io::Error::from)?;
        self.modified = false;
        Ok(())
    }

    pub fn data(&self) -> &C {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut C {
        self.modified = true;
        &mut self.data
    }
}

impl<C: Serialize> Drop for ConfigFile<C> {

    fn drop(&mut self) {
        if self.modified {
            if let Err(e) = self.save() {
                
            }
        }
    }
}