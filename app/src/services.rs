use anyhow::Result;

pub trait Request {
    type Resp;
}

pub trait Queryable<Req>
where
    Req: Request,
{
    fn query(&self, req: Req) -> Result<Req::Resp>;
}

pub trait Commandable<Req>
where
    Req: Request,
{
    fn execute(&self, req: Req) -> Result<Req::Resp>;
}
