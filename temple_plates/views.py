from pyramid.view import view_config


@view_config(route_name='menu', renderer='templates/menu.jinja2')
def menu(request):
    return {'project': 'temple-plates'}