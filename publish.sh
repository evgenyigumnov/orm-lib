#!/bin/bash
cd lib
cargo publish -p ormlib
cd ../orm_derive
cargo publish -p ormlib_derive