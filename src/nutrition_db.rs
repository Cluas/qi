//! 本地中式高频餐饮/饮品/补剂营养与节律词典（60+ 条目，支持模糊检索与一句话自然语言槽位提取）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodDictEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub aliases: &'static str,
    pub category: &'static str,
    pub kcal: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
    pub rhythm_tag: &'static str,
    pub tip: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodSearchHit {
    pub id: String,
    pub name: String,
    pub category: String,
    pub kcal: f64,
    pub protein_g: f64,
    pub carbs_g: f64,
    pub fat_g: f64,
    pub rhythm_tag: String,
    pub tip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedNaturalIntake {
    pub category: String,
    pub item_name: String,
    pub calories_kcal: Option<f64>,
    pub tags: Vec<String>,
    pub notes: String,
}

pub const FOOD_DICT: &[FoodDictEntry] = &[
    // --- 咖啡与茶饮 (10) ---
    FoodDictEntry { id: "iced_americano", name: "冰美式咖啡 (大杯无糖)", aliases: "冰美式,美式,黑咖啡,americano,咖啡", category: "caffeine", kcal: 12.0, protein_g: 0.3, carbs_g: 2.0, fat_g: 0.0, rhythm_tag: "晚间咖啡", tip: "15:00 后饮用易压缩深睡窗口" },
    FoodDictEntry { id: "coconut_latte", name: "生椰拿铁 (少糖)", aliases: "生椰拿铁,瑞幸,拿铁", category: "caffeine", kcal: 210.0, protein_g: 4.5, carbs_g: 22.0, fat_g: 11.0, rhythm_tag: "晚间咖啡", tip: "含椰浆饱和脂肪与咖啡因" },
    FoodDictEntry { id: "dirty_coffee", name: "Dirty 脏咖啡", aliases: "dirty,脏咖啡,浓缩牛奶", category: "caffeine", kcal: 185.0, protein_g: 6.0, carbs_g: 11.0, fat_g: 12.5, rhythm_tag: "晚间咖啡", tip: "浓缩双份提神" },
    FoodDictEntry { id: "oat_latte", name: "燕麦拿铁 (无蔗糖)", aliases: "燕麦拿铁,燕麦奶", category: "caffeine", kcal: 165.0, protein_g: 3.0, carbs_g: 24.0, fat_g: 6.0, rhythm_tag: "日常饮食", tip: "上午饮用较少影响夜间 HRV" },
    FoodDictEntry { id: "matcha_latte", name: "抹茶拿铁 (低糖)", aliases: "抹茶,绿茶拿铁", category: "caffeine", kcal: 170.0, protein_g: 6.5, carbs_g: 21.0, fat_g: 6.5, rhythm_tag: "日常饮食", tip: "含茶氨酸与微量咖啡因" },
    FoodDictEntry { id: "boya_milk_tea", name: "原叶鲜奶茶 (去冰三分糖)", aliases: "伯牙绝弦,霸王茶姬,鲜奶茶,奶茶", category: "caffeine", kcal: 235.0, protein_g: 5.5, carbs_g: 28.0, fat_g: 11.0, rhythm_tag: "晚间咖啡", tip: "原叶绿茶/红茶咖啡因较高，晚间慎喝" },
    FoodDictEntry { id: "lemon_tea", name: "手打鸭屎香柠檬茶 (少糖)", aliases: "柠檬茶,鸭屎香", category: "beverage", kcal: 160.0, protein_g: 0.2, carbs_g: 38.0, fat_g: 0.1, rhythm_tag: "高糖饮品", tip: "单杯含糖偏高且茶碱偏强" },
    FoodDictEntry { id: "oolong_tea", name: "无糖乌龙茶 / 三得利乌龙", aliases: "乌龙茶,无糖茶,冷泡茶", category: "beverage", kcal: 0.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 0.0, rhythm_tag: "清淡补水", tip: "零糖零脂，佐餐解腻佳品" },
    FoodDictEntry { id: "sparkling_water", name: "气泡水 / 苏打水 (无糖)", aliases: "气泡水,苏打水,矿泉水", category: "beverage", kcal: 0.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 0.0, rhythm_tag: "清淡补水", tip: "充足补水有助于尿酸代谢" },
    FoodDictEntry { id: "soy_milk", name: "现磨无糖豆浆 (350ml)", aliases: "豆浆,无糖豆浆", category: "beverage", kcal: 110.0, protein_g: 10.5, carbs_g: 5.0, fat_g: 5.0, rhythm_tag: "高蛋白轻食", tip: "优质植物蛋白" },

    // --- 酒类与宵夜重餐 (11) ---
    FoodDictEntry { id: "craft_beer", name: "精酿啤酒 / IPA (500ml)", aliases: "精酿,啤酒,ipa,扎啤,小酌", category: "alcohol", kcal: 245.0, protein_g: 2.0, carbs_g: 20.0, fat_g: 0.0, rhythm_tag: "晚间饮酒", tip: "酒精与嘌呤双重负荷，常致次日 RHR 升高" },
    FoodDictEntry { id: "red_wine", name: "干红葡萄酒 (150ml)", aliases: "红酒,干红,葡萄酒", category: "alcohol", kcal: 125.0, protein_g: 0.1, carbs_g: 3.8, fat_g: 0.0, rhythm_tag: "晚间饮酒", tip: "哪怕一杯也会压缩夜间 REM 与深睡" },
    FoodDictEntry { id: "baijiu", name: "白酒小酌 (50ml)", aliases: "白酒,茅台,五粮液,二锅头", category: "alcohol", kcal: 145.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 0.0, rhythm_tag: "晚间饮酒", tip: "次日 HRV 下探概率极高，建议多喝温水" },
    FoodDictEntry { id: "whisky_highball", name: "威士忌 Highball", aliases: "威士忌,highball,鸡尾酒", category: "alcohol", kcal: 135.0, protein_g: 0.0, carbs_g: 1.5, fat_g: 0.0, rhythm_tag: "晚间饮酒", tip: "睡前 4 小时避免酒精有助于心脏夜休" },
    FoodDictEntry { id: "spicy_hotpot", name: "川味红油火锅 (牛肉+毛肚+时蔬)", aliases: "火锅,麻辣火锅,红油火锅,毛肚", category: "late_meal", kcal: 960.0, protein_g: 52.0, carbs_g: 42.0, fat_g: 64.0, rhythm_tag: "夜间重餐", tip: "高油高盐高嘌呤，晚吃易抬升静息心率" },
    FoodDictEntry { id: "chaoshan_hotpot", name: "潮汕清汤牛肉火锅 (吊龙+青菜)", aliases: "潮汕牛肉,牛肉火锅,清汤火锅", category: "meal", kcal: 640.0, protein_g: 62.0, carbs_g: 28.0, fat_g: 28.0, rhythm_tag: "高蛋白轻食", tip: "选清汤少沙茶酱，是聚餐里非常友好的高蛋白餐" },
    FoodDictEntry { id: "bbq_skewers", name: "深夜炭火烧烤 (羊肉串+烤茄子)", aliases: "烧烤,烤串,夜宵烧烤,撸串", category: "late_meal", kcal: 820.0, protein_g: 44.0, carbs_g: 36.0, fat_g: 56.0, rhythm_tag: "夜间重餐", tip: "夜宵高脂辛辣显著推迟消化与降温" },
    FoodDictEntry { id: "crayfish", name: "麻辣 / 蒜蓉小龙虾 (一斤半)", aliases: "小龙虾,龙虾", category: "late_meal", kcal: 540.0, protein_g: 48.0, carbs_g: 18.0, fat_g: 30.0, rhythm_tag: "高嘌呤", tip: "高嘌呤海鲜贝甲类，尿酸关注人群适可而止" },
    FoodDictEntry { id: "fried_chicken", name: "韩式炸鸡配可乐", aliases: "炸鸡,汉堡,薯条", category: "late_meal", kcal: 880.0, protein_g: 38.0, carbs_g: 74.0, fat_g: 46.0, rhythm_tag: "夜间重餐", tip: "精制碳水+反式炸油易引血糖过山车" },
    FoodDictEntry { id: "malatang", name: "麻辣烫 / 骨汤冒菜 (多菜少丸子)", aliases: "麻辣烫,冒菜", category: "meal", kcal: 520.0, protein_g: 28.0, carbs_g: 44.0, fat_g: 24.0, rhythm_tag: "日常饮食", tip: "少喝重油汤底可省下 200kcal" },
    FoodDictEntry { id: "snail_noodles", name: "柳州螺蛳粉 (原味)", aliases: "螺蛳粉,米粉", category: "meal", kcal: 680.0, protein_g: 18.0, carbs_g: 86.0, fat_g: 28.0, rhythm_tag: "夜间重餐", tip: "高钠高油，午间食用优于晚间" },

    // --- 减脂高蛋白与轻食 (14) ---
    FoodDictEntry { id: "chicken_bowl", name: "杂粮鸡胸肉能量碗", aliases: "轻食,鸡胸,沙拉,能量碗,poke", category: "meal", kcal: 460.0, protein_g: 42.0, carbs_g: 45.0, fat_g: 11.0, rhythm_tag: "高蛋白轻食", tip: "稳血糖且夜间消化无负担" },
    FoodDictEntry { id: "salmon_poke", name: "三文鱼牛油果波奇饭", aliases: "三文鱼,波奇饭,刺身,日料", category: "meal", kcal: 540.0, protein_g: 34.0, carbs_g: 52.0, fat_g: 21.0, rhythm_tag: "高蛋白轻食", tip: "富含 Omega-3，友好心肺与血脂" },
    FoodDictEntry { id: "steamed_seabass", name: "清蒸海鲈鱼配时蔬糙米", aliases: "清蒸鱼,鲈鱼,蒸鱼", category: "meal", kcal: 420.0, protein_g: 44.0, carbs_g: 36.0, fat_g: 9.5, rhythm_tag: "高蛋白轻食", tip: "低脂优质白肉蛋白" },
    FoodDictEntry { id: "boil_shrimp", name: "白灼大虾 + 蒜蓉西兰花", aliases: "白灼虾,大虾,西兰花", category: "meal", kcal: 360.0, protein_g: 40.0, carbs_g: 22.0, fat_g: 8.0, rhythm_tag: "高蛋白轻食", tip: "超干净减脂晚餐组合" },
    FoodDictEntry { id: "cantonese_roast", name: "广式白切鸡/烧鹅拼时蔬饭 (去皮)", aliases: "白切鸡,烧腊,烧鸭饭", category: "meal", kcal: 580.0, protein_g: 36.0, carbs_g: 62.0, fat_g: 19.0, rhythm_tag: "日常饮食", tip: "去皮吃肉可大幅减少饱和脂肪" },
    FoodDictEntry { id: "beef_broccoli", name: "黑椒牛柳炒芦笋 (少油)", aliases: "牛柳,牛肉,芦笋,牛排", category: "meal", kcal: 490.0, protein_g: 38.0, carbs_g: 26.0, fat_g: 24.0, rhythm_tag: "高蛋白轻食", tip: "补铁补蛋白，力量训练日理想主菜" },
    FoodDictEntry { id: "scrambled_egg_tomato", name: "番茄炒蛋配二米饭 (家常少油)", aliases: "番茄炒蛋,西红柿炒蛋,家常菜", category: "meal", kcal: 480.0, protein_g: 19.0, carbs_g: 64.0, fat_g: 15.0, rhythm_tag: "日常饮食", tip: "经典温和家常味" },
    FoodDictEntry { id: "tofu_soup", name: "鲫鱼豆腐汤 / 菌菇豆腐煲", aliases: "豆腐,菌菇,汤", category: "meal", kcal: 310.0, protein_g: 24.0, carbs_g: 16.0, fat_g: 14.0, rhythm_tag: "高蛋白轻食", tip: "温润易消化" },
    FoodDictEntry { id: "greek_yogurt", name: "无蔗糖希腊酸奶碗 (+蓝莓坚果)", aliases: "希腊酸奶,酸奶,蓝莓", category: "meal", kcal: 240.0, protein_g: 18.0, carbs_g: 18.0, fat_g: 9.0, rhythm_tag: "高蛋白轻食", tip: "高蛋白低碳下午加餐" },
    FoodDictEntry { id: "protein_shake", name: "分离乳清蛋白粉 (1 勺加水)", aliases: "蛋白粉,乳清,shake", category: "supplement", kcal: 120.0, protein_g: 25.0, carbs_g: 2.0, fat_g: 1.2, rhythm_tag: "高蛋白轻食", tip: "训后 30 分钟快速补蛋白" },
    FoodDictEntry { id: "boiled_eggs", name: "水煮鸡蛋 2 枚 + 黑麦面包 1 片", aliases: "鸡蛋,水煮蛋,全麦,三明治", category: "meal", kcal: 295.0, protein_g: 18.0, carbs_g: 26.0, fat_g: 12.0, rhythm_tag: "高蛋白轻食", tip: "稳定早晨皮质醇与血糖曲线" },
    FoodDictEntry { id: "oatmeal", name: "热牛奶燕麦粥 + 奇亚籽", aliases: "燕麦,麦片,牛奶", category: "meal", kcal: 330.0, protein_g: 14.0, carbs_g: 46.0, fat_g: 9.0, rhythm_tag: "日常饮食", tip: "可溶性纤维友好 LDL-C 指标" },
    FoodDictEntry { id: "sushi_set", name: "日式三文鱼寿司拼盘 (8贯)", aliases: "寿司,三文鱼寿司", category: "meal", kcal: 490.0, protein_g: 26.0, carbs_g: 68.0, fat_g: 11.0, rhythm_tag: "日常饮食", tip: "醋饭升糖略快，先吃毛豆和刺身" },
    FoodDictEntry { id: "sub_sandwich", name: "全麦火鸡烤牛肉潜艇三明治", aliases: "赛百味,潜艇堡,牛肉三明治", category: "meal", kcal: 410.0, protein_g: 29.0, carbs_g: 46.0, fat_g: 11.0, rhythm_tag: "高蛋白轻食", tip: "选油醋汁代替蛋黄酱更轻盈" },

    // --- 中式经典主食与小吃 (15) ---
    FoodDictEntry { id: "lanzhou_ramen", name: "兰州清汤牛肉拉面 (多肉少面)", aliases: "牛肉面,拉面,兰州拉面", category: "meal", kcal: 560.0, protein_g: 26.0, carbs_g: 78.0, fat_g: 14.0, rhythm_tag: "日常饮食", tip: "少喝汤减少额外钠摄入" },
    FoodDictEntry { id: "pork_dumplings", name: "玉米鲜肉水饺 (15个)", aliases: "饺子,水饺", category: "meal", kcal: 540.0, protein_g: 22.0, carbs_g: 68.0, fat_g: 19.0, rhythm_tag: "日常饮食", tip: "配一碟凉拌木耳或青菜更均衡" },
    FoodDictEntry { id: "shengjian", name: "生煎包 4 个 + 牛肉粉丝汤", aliases: "生煎,生煎包,小笼包,包子", category: "meal", kcal: 610.0, protein_g: 21.0, carbs_g: 66.0, fat_g: 28.0, rhythm_tag: "日常饮食", tip: "表皮吸油较多，适合早午餐吃" },
    FoodDictEntry { id: "jianbing", name: "杂粮煎饼果子 (双蛋少薄脆)", aliases: "煎饼果子,杂粮煎饼,早餐饼", category: "meal", kcal: 450.0, protein_g: 18.0, carbs_g: 54.0, fat_g: 17.0, rhythm_tag: "日常饮食", tip: "不放辣条脆饼版本很抗饿" },
    FoodDictEntry { id: "roujiamo", name: "陕西腊汁肉夹馍 + 凉皮", aliases: "肉夹馍,凉皮", category: "meal", kcal: 720.0, protein_g: 24.0, carbs_g: 88.0, fat_g: 29.0, rhythm_tag: "高糖高碳", tip: "双碳组合易餐后犯困，午后宜快走 15 分钟" },
    FoodDictEntry { id: "cantonese_dimsum", name: "广式早茶 (虾饺+肠粉+白灼菜心)", aliases: "早茶,虾饺,肠粉,茶点", category: "meal", kcal: 510.0, protein_g: 25.0, carbs_g: 58.0, fat_g: 18.0, rhythm_tag: "日常饮食", tip: "虾饺配白灼菜心营养干净" },
    FoodDictEntry { id: "claypot_rice", name: "广式腊味滑鸡煲仔饭 (少酱)", aliases: "煲仔饭,滑鸡饭", category: "meal", kcal: 740.0, protein_g: 29.0, carbs_g: 86.0, fat_g: 29.0, rhythm_tag: "日常饮食", tip: "滑鸡优于肥腊肠" },
    FoodDictEntry { id: "wonton", name: "鲜虾小馄饨 + 卤蛋", aliases: "馄饨,云吞,云吞面", category: "meal", kcal: 420.0, protein_g: 22.0, carbs_g: 49.0, fat_g: 14.0, rhythm_tag: "日常饮食", tip: "轻暖易吸收" },
    FoodDictEntry { id: "rice_noodle", name: "云南过桥米线", aliases: "米线,过桥米线", category: "meal", kcal: 590.0, protein_g: 24.0, carbs_g: 76.0, fat_g: 19.0, rhythm_tag: "日常饮食", tip: "汤底表层鸡油略多，撇油更清淡" },
    FoodDictEntry { id: "congee_egg", name: "皮蛋瘦肉粥 + 清炒菜心", aliases: "瘦肉粥,白粥,粥", category: "meal", kcal: 340.0, protein_g: 16.0, carbs_g: 52.0, fat_g: 7.0, rhythm_tag: "清淡调养", tip: "肠胃需休息时温和不腻" },
    FoodDictEntry { id: "bibimbap", name: "韩式石锅拌饭 (去辣酱半份)", aliases: "拌饭,石锅拌饭", category: "meal", kcal: 610.0, protein_g: 23.0, carbs_g: 82.0, fat_g: 19.0, rhythm_tag: "日常饮食", tip: "含菌菇菠菜豆芽多种膳食纤维" },
    FoodDictEntry { id: "braised_pork", name: "红烧肉盖饭 / 卤肉饭", aliases: "红烧肉,卤肉饭,猪脚饭", category: "meal", kcal: 840.0, protein_g: 26.0, carbs_g: 84.0, fat_g: 42.0, rhythm_tag: "夜间重餐", tip: "饱腹但饱和脂肪偏高，宜配高纤蔬菜" },
    FoodDictEntry { id: "corn_sweet_potato", name: "蒸紫薯/贝贝南瓜 + 水果玉米", aliases: "紫薯,红薯,玉米,南瓜", category: "meal", kcal: 240.0, protein_g: 6.0, carbs_g: 52.0, fat_g: 1.5, rhythm_tag: "粗粮稳糖", tip: "优质慢碳替代精细白米" },
    FoodDictEntry { id: "nuts_mix", name: "每日原味综合坚果 (25g)", aliases: "坚果,核桃,巴旦木,腰果", category: "snack", kcal: 155.0, protein_g: 5.0, carbs_g: 4.5, fat_g: 13.5, rhythm_tag: "抗炎好脂", tip: "单不饱和脂肪酸护心血管" },
    FoodDictEntry { id: "blueberry_apple", name: "蓝莓 / 苹果 / 猕猴桃 1 份", aliases: "水果,苹果,蓝莓,猕猴桃,草莓", category: "snack", kcal: 85.0, protein_g: 1.0, carbs_g: 20.0, fat_g: 0.3, rhythm_tag: "低GI水果", tip: "富含多酚与维 C，温和抗氧化" },

    // --- 常见营养补剂与助眠节律 (10) ---
    FoodDictEntry { id: "mag_glycinate", name: "甘氨酸镁 (Magnesium Glycinate 200mg)", aliases: "甘氨酸镁,镁,补镁", category: "supplement", kcal: 0.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 0.0, rhythm_tag: "助眠支持", tip: "睡前 45 分钟服用有助于副交感神经放松" },
    FoodDictEntry { id: "vit_d3_k2", name: "维生素 D3 + K2 (2000 IU)", aliases: "维生素d,vd,d3", category: "supplement", kcal: 0.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 0.0, rhythm_tag: "晨间补剂", tip: "脂溶性维生素，随早餐或午餐同服吸收更好" },
    FoodDictEntry { id: "fish_oil", name: "高纯度深海鱼油 EPA+DHA (1000mg)", aliases: "鱼油,omega3,dha", category: "supplement", kcal: 10.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 1.0, rhythm_tag: "抗炎好脂", tip: "有助平稳血脂与高强度运动后恢复" },
    FoodDictEntry { id: "coq10", name: "辅酶 Q10 (泛醇 100mg)", aliases: "辅酶q10,q10", category: "supplement", kcal: 0.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 0.0, rhythm_tag: "晨间补剂", tip: "随早午餐脂肪食物同服支持线粒体活力" },
    FoodDictEntry { id: "theanine", name: "L-茶氨酸 (200mg 舒压)", aliases: "茶氨酸", category: "supplement", kcal: 0.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 0.0, rhythm_tag: "助眠支持", tip: "缓冲咖啡因心悸或助晚间放空" },
    FoodDictEntry { id: "melatonin_low", name: "低剂量褪黑素 (0.5–1mg)", aliases: "褪黑素", category: "supplement", kcal: 0.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 0.0, rhythm_tag: "倒时差助眠", tip: "仅建议倒时差或昼夜节律紊乱时短期调节" },
    FoodDictEntry { id: "creatine", name: "一水肌酸 (5g 力量储备)", aliases: "肌酸", category: "supplement", kcal: 0.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 0.0, rhythm_tag: "训练支持", tip: "注意日间多喝水 300ml" },
    FoodDictEntry { id: "electrolyte", name: "无糖电解质泡腾片/盐丸", aliases: "电解质,盐丸,宝矿力", category: "supplement", kcal: 15.0, protein_g: 0.0, carbs_g: 3.0, fat_g: 0.0, rhythm_tag: "训练补水", tip: "长跑或大汗后迅速回补钠钾镁" },
    FoodDictEntry { id: "b_complex", name: "活性复合维生素 B 族", aliases: "vb,b族,维生素b", category: "supplement", kcal: 0.0, protein_g: 0.0, carbs_g: 0.0, fat_g: 0.0, rhythm_tag: "晨间补剂", tip: "建议早晨随餐服用，晚间服用可能轻微提神" },
    FoodDictEntry { id: "probiotic", name: "复合益生菌 / 益生元", aliases: "益生菌,肠道", category: "supplement", kcal: 5.0, protein_g: 0.0, carbs_g: 1.0, fat_g: 0.0, rhythm_tag: "清淡调养", tip: "饭后温水送服支持肠道微生态" },
];

pub fn search_foods(query: &str, limit: usize) -> Vec<FoodSearchHit> {
    let q = query.trim().to_ascii_lowercase();
    let max_n = limit.max(1);
    FOOD_DICT
        .iter()
        .filter(|e| {
            if q.is_empty() {
                return true;
            }
            e.name.to_ascii_lowercase().contains(&q)
                || e.aliases.to_ascii_lowercase().contains(&q)
                || e.rhythm_tag.to_ascii_lowercase().contains(&q)
                || e.category.to_ascii_lowercase().contains(&q)
        })
        .take(max_n)
        .map(|e| FoodSearchHit {
            id: e.id.to_owned(),
            name: e.name.to_owned(),
            category: e.category.to_owned(),
            kcal: e.kcal,
            protein_g: e.protein_g,
            carbs_g: e.carbs_g,
            fat_g: e.fat_g,
            rhythm_tag: e.rhythm_tag.to_owned(),
            tip: e.tip.to_owned(),
        })
        .collect()
}

/// 确定性自然语言一句话槽位抽取（离线 0ms 完成；不依赖云端算力也不会误编热量）
pub fn parse_natural_intake_text(sentence: &str) -> Vec<ParsedNaturalIntake> {
    let raw = sentence.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    let lower = raw.to_ascii_lowercase();

    let is_late = raw.contains("夜宵")
        || raw.contains("宵夜")
        || raw.contains("睡前")
        || raw.contains("昨晚")
        || raw.contains("凌晨")
        || raw.contains("21:")
        || raw.contains("22:")
        || raw.contains("23:")
        || raw.contains("十一点")
        || raw.contains("十点");

    let is_afternoon_late = raw.contains("15:")
        || raw.contains("16:")
        || raw.contains("17:")
        || raw.contains("18:")
        || raw.contains("下午三")
        || raw.contains("下午四")
        || raw.contains("下午五")
        || raw.contains("傍晚");

    let mut matched: Vec<ParsedNaturalIntake> = Vec::new();
    for entry in FOOD_DICT {
        let alias_hit = entry
            .aliases
            .split(',')
            .any(|a| !a.is_empty() && lower.contains(&a.to_ascii_lowercase()));
        if alias_hit || lower.contains(&entry.name.to_ascii_lowercase()) {
            let mut tags = vec![entry.rhythm_tag.to_owned()];
            if is_late && !tags.iter().any(|t| t == "夜间重餐") && entry.category == "meal" {
                tags.push("夜间重餐".to_owned());
            }
            if (is_late || is_afternoon_late) && entry.category == "caffeine" {
                if !tags.iter().any(|t| t == "晚间咖啡") {
                    tags.push("晚间咖啡".to_owned());
                }
            }
            matched.push(ParsedNaturalIntake {
                category: if is_late && entry.category == "meal" {
                    "late_meal".to_owned()
                } else {
                    entry.category.to_owned()
                },
                item_name: entry.name.to_owned(),
                calories_kcal: if entry.kcal > 0.0 { Some(entry.kcal) } else { None },
                tags,
                notes: format!("一句话速记：{raw}"),
            });
            if matched.len() >= 3 {
                break;
            }
        }
    }

    if matched.is_empty() {
        let mut tags = Vec::new();
        let mut category = "meal";
        if raw.contains("咖啡") || raw.contains("拿铁") || raw.contains("美式") || raw.contains("茶") {
            category = "caffeine";
            tags.push(if is_late || is_afternoon_late { "晚间咖啡" } else { "日常提神" }.to_owned());
        } else if raw.contains("酒") || raw.contains("啤") || raw.contains("醉") {
            category = "alcohol";
            tags.push("晚间饮酒".to_owned());
        } else if is_late {
            category = "late_meal";
            tags.push("夜间重餐".to_owned());
        } else if raw.contains("镁") || raw.contains("维") || raw.contains("鱼油") {
            category = "supplement";
            tags.push("助眠支持".to_owned());
        } else {
            tags.push("日常饮食".to_owned());
        }
        matched.push(ParsedNaturalIntake {
            category: category.to_owned(),
            item_name: raw.to_owned(),
            calories_kcal: None,
            tags,
            notes: "一句话自由记录".to_owned(),
        });
    }

    matched
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_chinese_foods_and_parses_natural_sentence() {
        let hits = search_foods("火锅", 5);
        assert!(hits.len() >= 2);
        assert!(hits.iter().any(|h| h.name.contains("潮汕")));

        let parsed = parse_natural_intake_text("下午四点喝了杯生椰拿铁，晚上聚餐喝了精酿啤酒");
        assert!(parsed.len() >= 2);
        assert!(parsed.iter().any(|p| p.tags.contains(&"晚间咖啡".to_owned())));
        assert!(parsed.iter().any(|p| p.tags.contains(&"晚间饮酒".to_owned())));
    }
}
