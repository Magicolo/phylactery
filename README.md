<div align="center"> <h1> phylactery 0.2.0 </h1> </div>

<p align="center">
    <em> 

Safe wrappers around lifetime extension by splitting a `&'a T` into a `Lich<dyn T + 'b>` (`'b` can be any chosen lifetime) and a `Soul<'a>` which tracks the original lifetime.
On drop of the `Soul<'a>` or on calling `Soul::sever`, it is guaranteed that the captured reference is also dropped, thus
inaccessible from a remaining `Lich<T>`.
    </em>
</p>

<div align="right">
    <a href="https://github.com/Magicolo/phylactery/actions/workflows/test.yml"> <img src="https://github.com/Magicolo/phylactery/actions/workflows/test.yml/badge.svg"> </a>
    <a href="https://crates.io/crates/phylactery"> <img src="https://img.shields.io/crates/v/phylactery.svg"> </a>
</div>
