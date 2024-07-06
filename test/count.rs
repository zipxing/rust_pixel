#[derive(Debug)]
struct AlphaInfo {
    alpha: u8,
    count: u32,
}

fn main() {
    let txt = "My name is Zhou Chengyi,and you can call me Bill.I was borned in China and I speak Chinese.China,my home country,is an amazing country with a long history and a lot of traditional culture. You may  like to watch beautiful fireworks in a celebrating.However,you may not know that,fireworks comes from an small accident when alcheming long long ago in China.The home country of cute pandas is China,too.My home is in Hebei,Tangshan China.I have a big family which has four people in it.My mother is a officer worked in China Mobile.My father is a programmer.I also have a older brother,studied in a great university in China.And I,a middle school student.Well,there is more information about me,I have a lot of hobbies such as:playing basketball,drawing,and also listening to music.And I am also very glad to make new friends.I already have many friends in China,they are all very friendly however,I also need to communicate and making friends with persons that comes all over the world,including local people.Every one has their own favorite and least like.For me,I like to eat beef and noodles.They delicious and nutritious.Well, that is all the information about me.It is great to be interested about me ,about China and about Chinese culture.I want to get know about the culture all over the world welcome to communicate with me about your country.";
    let vtxt = txt.as_bytes();
    let vlen = vtxt.len();
    let mut vinfo: Vec<AlphaInfo> = vec![];
    for i in 0..26 {
        vinfo.push(AlphaInfo {
            alpha: 'a' as u8 + i as u8,
            count: 0,
        });
    }
    for i in 0..vlen {
        let va = vtxt[i] as char;
        if va.is_alphabetic() {
            vinfo[va.to_ascii_lowercase() as usize - 'a' as usize].count += 1;
        }
    }
    vinfo.sort_by(|b, a| a.count.cmp(&b.count));
    for o in &vinfo {
        println!("{:?}  {:?}", o.alpha as char, o.count);
    }
}
