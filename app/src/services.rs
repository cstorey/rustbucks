use failure::Fallible;

pub trait Request {
    type Resp;
}

pub trait Queryable<Req>
where
    Req: Request,
{
    fn query(self, req: Req) -> Fallible<Req::Resp>;
}
