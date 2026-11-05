//! Versioned PC storage adapter. Integers/floats retain their exact bits;
//! this container is deliberately distinct from Xbox memcard/DataSystem files.
use std::{ops::Range,sync::Arc};
pub(crate) type Result<T> = std::result::Result<T,String>;
#[derive(Default)]
pub(crate) struct Writer(pub Vec<u8>);
pub(crate) struct Reader<'a>{pub data:&'a [u8],pub offset:usize}
impl<'a> Reader<'a>{
    pub fn new(data:&'a [u8])->Self {Self{data,offset:0}}
    pub fn bytes(&mut self,n:usize)->Result<&'a [u8]> {
        let end=self.offset.checked_add(n).ok_or("replay length overflow")?;
        let data=self.data.get(self.offset..end).ok_or("truncated replay")?;
        self.offset=end;Ok(data)
    }
}
pub(crate) trait Store:Sized {
    fn put(&self,w:&mut Writer);
    fn get(r:&mut Reader<'_>)->Result<Self>;
}
macro_rules! number {($t:ty,$n:expr)=>{impl Store for $t {
    fn put(&self,w:&mut Writer){w.0.extend_from_slice(&self.to_be_bytes());}
    fn get(r:&mut Reader<'_>)->Result<Self>{Ok(<$t>::from_be_bytes(r.bytes($n)?.try_into().map_err(|_|"replay scalar width")?))}
}};}
number!(u8,1);number!(u16,2);number!(u32,4);number!(i32,4);number!(u64,8);number!(f32,4);
impl Store for usize {
    fn put(&self,w:&mut Writer){(*self as u64).put(w);}
    fn get(r:&mut Reader<'_>)->Result<Self>{usize::try_from(u64::get(r)?).map_err(|_|"replay index overflow".into())}
}
impl Store for bool {
    fn put(&self,w:&mut Writer){u8::from(*self).put(w);}
    fn get(r:&mut Reader<'_>)->Result<Self>{match u8::get(r)? {0=>Ok(false),1=>Ok(true),_=>Err("invalid replay boolean".into())}}
}
impl<T:Store> Store for Option<T> {
    fn put(&self,w:&mut Writer){self.is_some().put(w);if let Some(v)=self{v.put(w);}}
    fn get(r:&mut Reader<'_>)->Result<Self>{if bool::get(r)? {Ok(Some(T::get(r)?))}else{Ok(None)}}
}
impl<T:Store,const N:usize> Store for [T;N] {
    fn put(&self,w:&mut Writer){for v in self {v.put(w);}}
    fn get(r:&mut Reader<'_>)->Result<Self>{(0..N).map(|_|T::get(r)).collect::<Result<Vec<_>>>()?.try_into().map_err(|_|"replay array length".into())}
}
impl<T:Store> Store for Vec<T> {
    fn put(&self,w:&mut Writer){self.len().put(w);for v in self {v.put(w);}}
    fn get(r:&mut Reader<'_>)->Result<Self>{
        let n=usize::get(r)?;
        if n>r.data.len().saturating_sub(r.offset){return Err("replay count exceeds payload".into());}
        (0..n).map(|_|T::get(r)).collect()
    }
}
impl Store for String {
    fn put(&self,w:&mut Writer){self.as_bytes().len().put(w);w.0.extend_from_slice(self.as_bytes());}
    fn get(r:&mut Reader<'_>)->Result<Self>{let n=usize::get(r)?;String::from_utf8(r.bytes(n)?.to_vec()).map_err(|_|"replay string encoding".into())}
}
impl<T:Store> Store for Arc<[T]> {
    fn put(&self,w:&mut Writer){self.len().put(w);for v in self.iter(){v.put(w);}}
    fn get(r:&mut Reader<'_>)->Result<Self>{Ok(Vec::<T>::get(r)?.into())}
}
impl Store for Range<usize> {
    fn put(&self,w:&mut Writer){self.start.put(w);self.end.put(w);}
    fn get(r:&mut Reader<'_>)->Result<Self>{Ok(usize::get(r)?..usize::get(r)?)}
}
macro_rules! tuple {($($t:ident:$i:tt),+)=>{impl<$($t:Store),+> Store for ($($t,)+) {
    fn put(&self,w:&mut Writer){$(self.$i.put(w);)+}
    fn get(r:&mut Reader<'_>)->Result<Self>{Ok(($($t::get(r)?,)+))}
}};}
tuple!(A:0,B:1);tuple!(A:0,B:1,C:2);tuple!(A:0,B:1,C:2,D:3);

macro_rules! record {($t:ty {$($field:ident),* $(,)?})=>{impl Store for $t {
    fn put(&self,w:&mut Writer){$(self.$field.put(w);)*}
    fn get(r:&mut Reader<'_>)->Result<Self>{Ok(Self{$($field:Store::get(r)?,)*})}
}};
// Fields left out of the archive decode as their defaults.
($t:ty {$($field:ident),* $(,)?} ..default)=>{impl Store for $t {
    fn put(&self,w:&mut Writer){$(self.$field.put(w);)*}
    fn get(r:&mut Reader<'_>)->Result<Self>{Ok(Self{$($field:Store::get(r)?,)* ..Default::default()})}
}};}
pub(super) use record;
