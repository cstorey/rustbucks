use byteorder::{BigEndian, WriteBytesExt};
use failure::Error;
use futures::future::{lazy, poll_fn};
use futures::Future;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::Arc;
use tokio_threadpool::blocking;

use warp;
use warp::Filter;
use {error_to_rejection, render, WithTemplate};

#[derive(Serialize, Debug, Clone, Hash)]
pub struct Coffee {
    name: String,
}

#[derive(Debug, Clone)]
pub struct Menu {
    drinks: Arc<HashMap<Id, Coffee>>,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
struct Id {
    val: [u8; 16],
}

impl Id {
    fn of<H: Hash>(entity: &H) -> Self {
        let mut val = [0u8; 16];
        {
            let mut cursor = io::Cursor::new(&mut val as &mut [u8]);
            for i in 0..2 {
                let mut h = siphasher::sip::SipHasher24::new_with_keys(0, i as u64);
                entity.hash(&mut h);
                cursor
                    .write_u64::<BigEndian>(h.finish())
                    .expect("write_u64 to fixed size buffer should never fail");
            }
        }
        Id { val }
    }
}

impl fmt::Display for Id {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut buf = [0u8; 22];
        let sz = base64::encode_config_slice(&self.val, base64::URL_SAFE_NO_PAD, &mut buf);
        assert_eq!(sz, buf.len());
        write!(fmt, "{}", String::from_utf8_lossy(&buf))?;
        Ok(())
    }
}

impl std::str::FromStr for Id {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let mut id = Id::default();
        let sz = base64::decode_config_slice(src, base64::URL_SAFE_NO_PAD, &mut id.val)?;
        if sz != std::mem::size_of_val(&id.val) {
            bail!("Could not decode id from base64: {:?}", src)
        }
        Ok(id)
    }
}

#[derive(Debug, WeftRenderable)]
#[template(path = "src/menu/menu.html")]
struct MenuWidget {
    drink: Vec<(Id, Coffee)>,
}
#[derive(Debug, WeftRenderable)]
#[template(path = "src/menu/drink.html")]
struct DrinkWidget {
    id: Id,
    drink: Coffee,
}

// impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection>
impl Menu {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        Self::insert(
            &mut map,
            Coffee {
                name: "Umbrella".into(),
            },
        );
        Self::insert(
            &mut map,
            Coffee {
                name: "Fnordy".into(),
            },
        );
        Menu {
            drinks: Arc::new(map),
        }
    }

    fn insert(map: &mut HashMap<Id, Coffee>, drink: Coffee) {
        let id = Id::of(&drink);
        let prev_size = map.len();
        map.insert(id, drink);
        assert!(map.len() > prev_size);
    }

    pub fn handler(
        &self,
    ) -> impl warp::Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> {
        let me = self.clone();
        let index = warp::get2()
            .and(warp::path::end())
            .and_then(move || error_to_rejection(me.index()))
            .and_then(render);
        let me = self.clone();
        let details = warp::get2()
            .and(warp::path::path("menu"))
            .and(warp::path::param::<Id>())
            .and(warp::path::end())
            .and_then(move |id| me.detail(id))
            .and_then(render);
        index.or(details)
    }

    fn index(&self) -> impl Future<Item = WithTemplate<MenuWidget>, Error = failure::Error> {
        info!("Handle index");
        info!("Handle from : {:?}", ::std::thread::current());
        self.load_menu()
            .map_err(|e| failure::Error::from(e))
            .map(|menu| {
                info!("Resume from : {:?}", ::std::thread::current());
                WithTemplate {
                    value: MenuWidget { drink: menu },
                }
            })
    }

    fn detail(
        &self,
        id: Id,
    ) -> impl Future<Item = WithTemplate<DrinkWidget>, Error = warp::Rejection> {
        error_to_rejection(self.load_drink(id))
            .and_then(|drinkp| drinkp.ok_or_else(warp::reject::not_found))
            .map(move |drink| WithTemplate {
                value: DrinkWidget {
                    id: id,
                    drink: drink,
                },
            })
    }

    fn load_menu(&self) -> impl Future<Item = Vec<(Id, Coffee)>, Error = failure::Error> {
        let me = self.clone();
        lazy(|| {
            poll_fn(move || {
                blocking(|| {
                    me.drinks
                        .iter()
                        .map(|(id, d)| (id.clone(), d.clone()))
                        .collect::<Vec<(Id, Coffee)>>()
                })
            }).map_err(Error::from)
        })
    }

    fn load_drink(&self, id: Id) -> impl Future<Item = Option<Coffee>, Error = failure::Error> {
        let me = self.clone();
        lazy(move || {
            poll_fn(move || blocking(|| me.drinks.get(&id).map(|d| d.clone()))).map_err(Error::from)
        })
    }
}

impl MenuWidget {
    fn drinks<'a>(&'a self) -> impl 'a + Iterator<Item = &'a (Id, Coffee)> {
        self.drink.iter()
    }
}
