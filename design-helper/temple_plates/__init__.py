from pyramid.config import Configurator


def main(global_config, **settings):
    """ This function returns a Pyramid WSGI application.
    """
    config = Configurator(settings=settings)
    config.include('pyramid_jinja2')
    config.add_static_view('static', 'static', cache_max_age=3600)
    config.add_route('drink', '/menu/{id}')
    config.add_route('new_order', '/orders')
    config.add_route('menu', '/')
    config.scan()
    return config.make_wsgi_app()
