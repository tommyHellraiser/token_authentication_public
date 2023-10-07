# user_token_authentication

Disclaimer: This software is not production-ready, it needs a lot more testing and security 
review before it can be used in production.

The idea for this app was to practice API design, authentication, and backend organization,
communication with a MySQL database. Use it only as an example, since that's what it was
created for.

Having said that, let's take a look at this app.

## How to use
This is a back-end only application, and as such, it does not have a GUI to use, or front-end. Testing 
and usage is achieved through analyzing the database and using a request app such as Postman. There's a
collection included for testing, feel free to use it and modify it as you see fit!

First of all, you'll need to have Rust cargo installed to run the project, run it with `cargo run`, 
and if you want to make an .exe file, so you don't have to execute it with cargo, run 
`cargo build --release` and cargo will compile a standalone release exe file for you.

## How and what to configure

First of all, you'll need a MySQL service running in your machine. Have a database created 
that's empty, because this app can (if configured to do so) drop it and create it again. By 
default, it will create a database with the name `user_token_authentication`, but you can change 
the name in the json config file to access the correct database, and also the ``schema_reset.sql`` 
file to create a database with the name you want.  

There's a json configuration file in the config folder, with the following parameters:

````JSON

{
  "service_url": "127.0.0.1",
  "service_port": "8010",
  "db_url": "mysql://root@localhost:3306/user_token_authentication",
  "reset_db": false
}

````

Simply put, `service_url` is the IP where your Http server is going to be listening. 
`service_port` is the port, `db_url` is the URL of your MySQL database and the name of 
the database you'll be using.

The parameter `reset_db` will drop the database at the start of execution and create 
it with the only two tables this app contains. 

Feel free to modify these parameters, and add to  the database structure if you want, but 
changing the names of the tables and types of the columns will cause this app to crash.

### IMPORTANT NOTICE ON RUNNING THIS APP
This backend app is designed to run with open ssl certificates, the Http server will not 
run if you don't include your certificate and your key in the ``certs`` folder. The program's
going to panic! if you don't include them. You should generate your own certificates having 
installed OpenSSL in your computer first. I won't include mine for clear reasons.

The paths for these files are: `certs/cert.pem` and `certs/key.pem`.

### Advice in setting the address for your Http server
I recommend using the localhost IP address: `127.0.0.1` instead of using the word `localhost`,
because when specifying the localhost name, the app will check first the IPv6 address, and
then the IPv4, which will make execution slower. It's not noticeable in a small app like this one,
but it's a good thing to know for bigger projects.

## Authentication method
As the name suggests, this app uses a token-based authentication. If you're not familiar with it, 
here's a quick description:

The user logs in (or creates and account in case they don't have one), using their username and 
password. The username is store in the database, along with some other info. The password is 
handled in a safe manner, not even stored directly for security reasons (password handling is 
mentioned in the next section).

When the user is logged in (or the account created, which triggers a login automatically),
the app generates a session token returned to the user in the Http response.
That session token will have a lifetime of 30 minutes, after which it will expire. The expiry logic
is handled by a cron service (more on that in a bit).

That session token that the user receives, should be stored to perform any other operations in this 
app, since any attempt to access a private endpoint will be checked for authentication using by using
an authentication middleware (also, more on that later).

## Password handling
The users module contains a password handler method that hashes the password entered by the user using the
default hasher. I won't upload my own personal method for obvious security reasons, but this can serve as an
example of how hashing and handling sensitive user data could be done.

## Users and permissions
There are some perks to using the superuser account, and they include:
- Creating an account with any amount of privileges (except for super of course, we can't have two superusers).
- Deleting other accounts.
- Forcefully stopping the Http server.
- Restore a deleted account.
- Changing another user's level (changing another user's password would be a good feature to have. Future feature).

More details on that are mentioned in the endpoint's documentation.

## API structure
For the API, I've included a Postman collection with the endpoints, so you can import it and
test them. The basic idea is that there's one and only one superuser, and there can be any amount of 
regular to admin (High level) users. The superuser cannot create another superuser, you can only do it
manually in the database (there could be checks to avoid this, but that'll probably be added in a
future version).

The users can have different permissions, ranging from 0 to 4, being 0 View only and 4 superuser.

This mention of the superuser and permissions is important because some functions are only available
for admins and superusers. Now, the structure of the API is the following:

- api/
  - public/
    - alive
  - internal/
    - alive
    - stop
    - stop_now
- users/
  - user_login
  - user_logout
  - create_user
  - manage/
    - change_password
    - delete_user
    - check_password
- internal/
  - create_user
  - delete_user_internal
  - undo_delete_user
  - change_user_level


Meaning that if you want to make a request to the ``delete_user`` endpoint under management, 
you should send a request to:

`{{AppUrl}}:{{AppPort}}/users/manage/delete_user`

The methods of the requests are in the Postman collection, as well as in the documentation inside
the project. There are also details of what you need to send in terms of headers and body.

Now, permissions-related subject, it was important because the endpoints under the ``internal``
paths are only accessible to High and Super level users. The ``manage`` path is only accessible to Low
level users and above. The ``api/internal/`` path is available to High users and above, and the 
``api/internal/stop_now`` endpoint is only available to Super level users.

The `internal` path for example, is available only to High and Super level users. There are some key 
differences between some of the endpoints for regular and higher level users, I encourage you
to go check out the code for more details!

This privileges checking is done via an Actix Web middleware, basically copied from their website and 
modified to fit this app's needs. The website for that example is:

https://actix.rs/docs/middleware/

### Brief details on the API endpoints:
- api/public/alive -> check the alive state of the service
- api/internal/alive -> same as with the public but private for testing purposes
- api/internal/stop -> stops the Http server gracefully, meaning that it'll wait for any other processes,
  threads or tasks to finish before closing the service
- api/internal/stop_now -> stops the server immediately, it won't wait for any process
- users/user_login -> logs the user in and returns a session token
- users/user_logout -> logs the user out and closes the session in runtime static ref and in database
- users/create_user -> creates a new user and returns a session token. If authenticated, it'll create a new user
  with one level below the user that's making the request. If not authenticated, it'll create a new user with
  level 1 (Low level). Once the user's been created, it'll trigger a login automatically.
- users/manage/change_password -> changes the password of the user making the request
- users/manage/delete_user -> deletes the user making the request and closes the session
- users/manage/check_password -> checks if the password entered by the user making the request is correct.
  It might be used when the user is prompted to "confirm their password", since it's a pretty lightweight service
  to execute
- internal/create_user -> creates a new user with the level specified in the request body. Only available to High
  and Super users. If a level was not sent in the request body, it'll create a user with one level below the
  requesting user's.
- internal/delete_user_internal -> deletes the user specified in the request body. Only available to High and Super.
  the user to be deleted can be at most, one level below the requesting user. A Super user can only delete up to High
  level users. High level users can only delete users up to Medium level, and so on.
- internal/undo_delete_user -> restores a deleted user. Only available to High and Super users.
- internal/change_user_level -> changes the level of the user specified in the request body to the level also
  specified in the request body. Only available to High and Super users. The new level for the user can be at most,
  one level below the requesting user's. Same previous example applies here.

## Cron service for auto session managing
I included a small but necessary cron that'll periodically check the status of the sessions in the database,
and will close the ones that are expired as soon as it detects them. It will also update the runtime status
of these sessions.

There's a static reference to a Sessions map that keeps track of the open and closed 
sessions. Its purpose is to be able to have an easy and fast-to-access means to a session index.
In this app, every session status is backed against this runtime reference first and the database afterward,
but I believe this cron system is reliable enough to use it as a primary source for session status checking.

Of course, more security will always be better, and so there could be checks to determine if the cron system
got shut down, so it can be started again, but again, those features might come in future versions. 
This is merely a testing app, and so there are a lot of details to polish.

## Some other details
There are some other things worth mentioning that are not the central idea of the app, but are a part of it
nonetheless:
- The ``config.json`` file is loaded at the start of execution and not reloaded in any other point. I will include
a hot reload feature down the line, but for now, to reload the config, just re-start the app.
- The environment configuration is saved in a static reference accessible anywhere in the program via a static 
reference instance and a Read/Write lock by Tokio (special thanks to them, greatest crate in the Rust ecosystem).
- The connection to the database is achieved through the mysql_async crate.
- The full API is based on actix-web.
- Json serialization and deserialization is achieved with the help or our good ol' serde and serde_json crates.
- There's a shutdown static reference that at the moment, only serves the purpose of letting the user know that the
service is shutting down, if the user happens to send an alive request while shutdown is in progress. In the future,
that static reference could have a much bigger use, such as telling the app if a service or heavy process is safe to 
start or not, based on the shutdown state.
- There are some macros used inside the FromRow implementation to extract certain values from the row element when 
selecting from database.
- There's an empty ``web_local`` module that was meant to hold requests to another service, but I got so into 
developing this app that I forgot to create another service. Stay tuned for that!
- Last but not least, I'm using a crate of mine for error conversion and propagation, called ``error_mapper``. It also
needs more refining and some macros implementations for better handling, conversion and creation of errors, but it's 
well on its way and very much usable for easy testing purposes. It provides easy conversion from the supported crates'
errors, and really easy propagation when you need to handle the error upstream.
 
If you want to check it out, and give me some feedback, I'd be more than happy to receive it, so I can further improve 
it and increase its utility for anyone who wants to try it out:

https://crates.io/crates/error_mapper

I also have pending to implement a logger crate and upload it to ``crates.io``, I'll do that in the near future and
include that logger in this project.

## Wrap-up
That sums this app up, I hope you find it useful, and if you have any questions, feel free to contact me at:

email: ``nacho.ponce25@gmail.com``

telegram: ``@tommyHellraiser``

Thank you for reading, and have a great one!