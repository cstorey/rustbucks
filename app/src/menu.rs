use warp;
use WithTemplate;

#[derive(Serialize, Debug, WeftTemplate)]
#[template(path = "src/menu/coffee.html")]
pub struct Coffee {
    id: u64,
    name: String,
}

pub fn index() -> Result<WithTemplate<Coffee>, warp::Rejection> {
    index_impl().map_err(|e| warp::reject::custom(e.compat()))
}

fn index_impl() -> Result<WithTemplate<Coffee>, failure::Error> {
    info!("Handle index");
    let res = WithTemplate {
        name: "template.html",
        value: Coffee { id: 42, name: "Umbrella".into() },
    };
    Ok(res)
}
