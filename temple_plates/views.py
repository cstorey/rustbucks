from pyramid.view import view_config


@view_config(route_name='menu', renderer='templates/menu.jinja2')
def menu(request):
    return {}

@view_config(route_name='drink', renderer='templates/drink.jinja2')
def drink(request):
    drink_id=request.matchdict['id']
    return dict(drink_id=drink_id)

@view_config(route_name='new_order', renderer='templates/order.jinja2')
def new_order(request):
    print(('matches', request.matchdict))
    print(('params', request.params))
    drink_id=request.params['drink_id']
    return dict(drink_id=drink_id)