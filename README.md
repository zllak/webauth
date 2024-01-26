<h1 align="center">webauth</h1>
<p align="center">Implements Authentication primitives for Rust</p>
<div align="center">
    <a href="https://crates.io/crates/webauth">
        <img src="https://img.shields.io/crates/v/webauth.svg" />
    </a>
    <a href="https://docs.rs/webauth">
        <img src="https://docs.rs/webauth/badge.svg" />
    </a>
</div>

## Overview
TODO

## Features
TODO

## Security and Standards
TODO

## Implementing a custom `Store`

We use resources like `Session`, `User`, ... and you can provide your own `Store`
implementation for those resources.
We provide some implementations but maybe you want to roll your own `User`, or
you use another database we have not implemented.

All you need is to implement the trait `Store`.

## Contributing
TODO
