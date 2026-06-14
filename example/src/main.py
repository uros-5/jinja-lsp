from fastapi.responses import HTMLResponse
from jinja2 import Environment, FileSystemLoader
from fastapi.templating import Jinja2Templates
from fastapi import FastAPI, Request

app = FastAPI()

def main():
    jinja_env = Environment(loader=FileSystemLoader("templates"))
    jinja_env.globals["PROJECT_NAME"] = "Hello, world!"
    template = jinja_env.get_template("account.jinja")
    _result = template.render(
        first_name="John",
        last_name="Doe",
        email="johndoe@example.com",
        phone_number="(123) 456-7890",
        street="123 Main St",
        city="Dallas",
        header_info="This is some information about the user.",
    )


@app.get("example", response_class=HTMLResponse)
async def fastapi_example(request: Request):
    templates = Jinja2Templates("templates")
    template = templates.TemplateResponse(request, name="account.jinja", context={"example": 11})
    # _response = template.render()

