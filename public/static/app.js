// the db_url and grouped is retrieved from the indexed db
function init(){
    var username = null;
    var password = null;
    if (window.localStorage){
        username = localStorage.getItem('username');
        password = localStorage.getItem('password');
        console.log("username:", username);
        console.log("password:", password);
    }
    else{
        console.error("localStorage is not supported");
    }
    var cred = null;
    if (username != null && password != null) {
        cred = {username: username, password: password}
    }
    app = Elm.Main.fullscreen(
        { login_required: false,
          db_name: null,
          api_endpoint: null,
          grouped: true,
          cred: cred 
        }
    );

    app.ports.title.subscribe(function(title) {
        document.title = title;
    });

    app.ports.setUsername.subscribe(function(username) {
        if (window.localStorage){
            localStorage.setItem('username', username);
        }
        else{
            console.error("localStorage is not supported");
        }
    });
    app.ports.setPassword.subscribe(function(password) {
        if (window.localStorage){
            localStorage.setItem('password', password);
        }
        else{
            console.error("localStorage is not supported");
        }
    });
}
window.onload = init
