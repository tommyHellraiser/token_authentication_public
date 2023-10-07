DROP DATABASE IF EXISTS `user_token_authentication`;
CREATE DATABASE `user_token_authentication`;
USE `user_token_authentication`;

DROP TABLE if EXISTS users;
CREATE TABLE users(
    ID INT PRIMARY KEY,
    username VARCHAR(20) UNIQUE KEY NOT NULL,
    hashed_pass VARCHAR(25) NOT NULL,
    email VARCHAR(50) NOT NULL,
    level ENUM('View', 'Low', 'Medium', 'High', 'Super') NOT NULL DEFAULT 'View',
    created_at DATETIME NOT NULL DEFAULT CURTIME(),
    updated_at DATETIME NOT NULL DEFAULT CURTIME(),
    deleted_at DATETIME DEFAULT NULL
);

DROP TABLE if EXISTS users_sessions;
CREATE TABLE users_sessions (
	users_ID INT UNIQUE KEY,
	token VARCHAR(45) NOT NULL,
	creation DATETIME NOT NULL,
	expiry DATETIME NOT NULL,
	FOREIGN KEY users_sessions_users_ID (users_ID) REFERENCES users (ID)
);
