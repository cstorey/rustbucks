from pyramid.view import view_config


@view_config(route_name='menu', renderer='templates/menu.jinja2')
def menu(request):
    return {}

@view_config(route_name='drink', renderer='templates/drink.jinja2')
def drink(request):
    drink_id=request.matchdict['id']
    return dict(drink_id=drink_id)