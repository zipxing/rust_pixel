#!/usr/bin/env python3
"""
Migrate symbols.png from 2048x2048 to 4096x4096 layout and add CJK characters.

Old 2048x2048 Layout:
- Rows 0-1535 (1536px): Sprite region, 6 rows × 8 blocks = 48 blocks, 12288 sprites
- Rows 1536-2047 (512px): TUI (blocks 48-52) + Emoji (blocks 53-55)

New 4096x4096 Layout (10 Sprite Rows):
- Rows 0-2559 (2560px): Sprite region, 10 rows × 16 blocks = 160 blocks, 40960 sprites
- Rows 2560-3071 (512px): TUI (blocks 160-169, x=0-2559) + Emoji (blocks 170-175, x=2560-4095)
- Rows 3072-4095 (1024px): CJK region, 128×32 grid, 4096 chars

Usage:
    python migrate_texture.py [options]

Options:
    --input PATH      Input 2048x2048 texture (default: assets/pix/symbols.png)
    --output PATH     Output 4096x4096 texture (default: assets/pix/symbols_4096.png)
    --font PATH       TTF font for CJK rendering (default: auto-detect)
    --chars PATH      File with CJK characters to render (default: built-in common chars)
    --no-cjk          Skip CJK rendering
    --preview         Generate a preview image showing regions
"""

import argparse
import os
import sys
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    print("Error: Pillow is required. Install with: pip install Pillow")
    sys.exit(1)


# Common CJK characters (GB2312 Level 1 most frequently used + game UI terms)
# Total: ~3000 characters covering most use cases
COMMON_CJK_CHARS = """
的一是不了在人有我他这中大来上个国和到说们为子时道要出就分对成会可
主发年动同工也能下过子说产种面而方后多定行学法所民得经十三之进着等
部度家电力里如水化高自二理起小物现实加量都两体制机当使点从业本去把
性好应开它合还因由其些然前外天政四日那社义事平形相全表间样与关各重
新线内数正心反你明看原又么利比或但质气第向道命此变条只没结解问意建
月公无系军很情最手世持什者战取非并内该科代王然第气次品比按展风求海
图计已山通入号毛至达走积示议象力马并元给识名百利济院信联更文常听火
组六往便观住象标张直各市感局许离特万光路更少南被局照专且头队车五清
管业题程展社白技运断真场将斯际才备叫记争共基金花群南深论目却华活总
府规术红难思严流带每东增则完决华党委调克用青选志收至治参再任保委则
西干器领空际先决交共响身热究式效土单革完立位统做资转强设阶流容器近
受边太历队导市己影且保精据联史即易改林究快群今期父像切明步算争转北
置价青装色德局切候古准类争七九除商整例始战门负属局角验服收断深际站
即书必走拉办片积故层速品济打素治段低极任院构广段片压低养源维调段较
态武满农另周维民备已言府包布讲首视满找望布劳达府确米养批复城故百越
放验拿细居约细包苏持司送察角另观队周根顾建火济维求布讲导落则底命议
球供认众议算离维写传写省施刘维约验另顾府施周布济落毫确注举界权则底
城称势往推伤善众欢投福负造城思织终卫律众房室施养省争论批连级包响济
达准类达精府达率拿状精府层类类团细精江级养处包刘米刘批组导持府导周
游戏开始结束返回确认取消等级分数时间生命攻击防御速度技能道具装备武
器防具头盔护甲战士法师弓箭手刺客魔法火球冰冻雷电治疗复活传送飞行跳
跃跑步攀爬游泳潜水钓鱼采集制作合成分解强化升级突破进阶觉醒神器传说
史诗稀有普通背包仓库商店任务副本地图世界boss怪物宠物坐骑飞行器建筑
城堡村庄城镇港口沙漠森林雪山火山洞穴遗迹迷宫竞技场战场公会家园好友
邮件聊天交易拍卖行充值商城礼包活动签到每日周常月卡年卡钻石金币银两
铜板经验声望荣誉积分排行榜成就称号头衔勋章徽章旗帜图腾阵营势力种族
人类精灵矮人兽人亡灵恶魔天使龙族妖怪鬼怪怪兽boss首领守卫巡逻士兵
将军元帅国王王后公主王子勇士英雄冒险家探险者猎人渔夫农民工匠商人医
生教师学者科学家艺术家音乐家画家诗人作家导演演员明星偶像粉丝观众玩
家新手老手高手大神萌新小白菜鸟大佬土豪平民百姓贵族皇室神仙佛祖道士
和尚尼姑修女牧师神父主教教皇圣女魔女巫师魔导士召唤师元素师炼金术士
符文师刻印师铭文师附魔师锻造师裁缝师炼药师采药师草药师猎魔人驱魔师
退魔师除灵师阴阳师风水师占卜师预言家先知使徒圣骑士暗黑骑士死亡骑士
龙骑士狮鹫骑士飞龙骑士战争机器攻城车投石机弩车炮台城墙护城河吊桥大
门暗门密道地道地下城堡秘密基地藏宝洞宝箱钥匙锁链机关陷阱毒针地刺尖
刺火焰冰霜雷电毒雾迷雾黑暗光明混沌秩序善恶正邪阴阳五行金木水火土风
雷冰毒圣暗无属性物理魔法远程近战单体群体持续瞬发蓄力吟唱施法冷却回
复消耗触发被动主动自动手动切换模式设置选项音量亮度画质特效阴影抗锯
齿垂直同步帧率分辨率全屏窗口无边框语言中文英文日文韩文法文德文西班
牙文俄文葡萄牙文意大利文阿拉伯文泰文越南文印尼文马来文繁体简体字幕
配音原声背景音乐音效环境声脚步声技能声语音通话麦克风耳机扬声器振动
触感反馈自动存档手动存档读取存档删除存档新建角色删除角色服务器列表
登录注册找回密码修改密码绑定手机绑定邮箱实名认证防沉迷未成年人保护
家长监护客服中心帮助文档常见问题反馈建议举报投诉封号申诉解封账号安
全二次验证验证码图形验证滑动验证短信验证邮箱验证人脸识别指纹识别声
纹识别安全问题安全密码支付密码交易密码锁仓锁定解锁冻结解冻封禁解禁
禁言禁止允许同意拒绝确定取消是否对错真假有无多少大小长短高低快慢轻
重厚薄粗细宽窄深浅明暗冷热干湿软硬滑涩甜苦酸辣咸鲜香臭美丑善恶好坏
新旧老少男女父母兄弟姐妹夫妻子女祖孙朋友亲人敌人对手队友同伴伙伴搭
档组队单人双人多人无限制限时限量首次唯一独占共享公开私有隐藏显示开
启关闭打开关上进入退出加入离开连接断开在线离线忙碌勿扰隐身上线下线
一二三四五六七八九十百千万亿零整半数量单位个只条张块片支把件套副对
双组群批次回趟遍番轮场局盘局面阶段回合轮次波次层次级别段位星级品级
品质稀有度掉落概率刷新时间冷却时间持续时间生效时间失效时间有效期限
永久临时限定特殊普通高级超级终极究极神级仙级圣级魔级鬼级王级帝级皇
霸神圣魔鬼仙佛妖怪龙凤虎狼狮豹熊狐猫狗兔鼠牛马羊猪鸡鸭鹅鱼虾蟹蛇
蝎蜘蛛蜜蜂蝴蝶蜻蜓蚂蚁蟑螂老鼠乌龟青蛙蜥蜴鳄鱼恐龙猛犸剑齿虎三叶
虫菊石鹦鹉螺角龙翼龙霸王龙迅猛龙甲龙剑龙蛇颈龙鱼龙沧龙始祖鸟猿人
智人现代人外星人机器人仿生人克隆人变异人改造人超能力者念力者心灵感
应者透视者预知者时间旅行者空间跳跃者维度穿越者平行宇宙多元宇宙虫洞
黑洞白洞星门传送门次元门结界屏障护盾防护罩能量罩力场重力场磁场电场
辐射核能太阳能风能水能地热能生物能机械能化学能热能声能光能电能磁能
暗能量暗物质反物质正物质原子分子离子电子质子中子光子声子引力子希格
斯玻色子夸克胶子介子重子轻子中微子暗物质粒子假想粒子虚粒子实粒子稳
定粒子不稳定粒子基本粒子复合粒子费米子玻色子标准模型弦理论膜理论圈
量子引力超对称超弦理论万有理论统一场论相对论量子力学热力学电动力学
经典力学牛顿力学拉格朗日力学哈密顿力学统计力学流体力学固体力学弹性
力学塑性力学断裂力学疲劳力学振动力学波动力学声学光学电学磁学热学化
学生物学生理学病理学药理学毒理学免疫学遗传学进化论细胞学分子生物学
生物化学生物物理学神经科学认知科学心理学社会学经济学政治学历史学地
理学天文学宇宙学数学几何学代数学分析学概率论统计学运筹学控制论信息
论系统论博弈论决策论图论拓扑学微分方程偏微分方程积分方程泛函分析实
分析复分析调和分析数值分析计算数学应用数学纯粹数学离散数学组合数学
算术几何代数分析拓扑概率统计逻辑集合论模型论证明论递归论计算复杂性
理论算法分析数据结构程序设计软件工程系统架构网络安全人工智能机器学
习深度学习神经网络自然语言处理计算机视觉语音识别图像处理信号处理模
式识别数据挖掘知识发现专家系统推理引擎规则引擎决策支持系统智能代理
多智能体系统分布式系统并行计算云计算边缘计算雾计算量子计算光子计算
神经形态计算类脑计算生物计算分子计算细胞计算遗传算法蚁群算法粒子群
算法模拟退火遗传编程进化策略差分进化人工蜂群算法萤火虫算法蝙蝠算法
灰狼优化鲸鱼优化花朵授粉布谷鸟搜索和声搜索禁忌搜索变邻域搜索局部搜
索全局搜索随机搜索确定性搜索启发式搜索元启发式搜索超启发式搜索混合
启发式搜索自适应启发式搜索多目标优化约束优化无约束优化连续优化离散
优化整数规划线性规划非线性规划动态规划贪心算法分治算法回溯算法分支
限界广度优先深度优先迭代加深双向搜索最佳优先一致代价启发搜索迭代加
深启发搜索递归最佳优先简化记忆限制山峰爬升模拟退火禁忌搜索遗传算法
差分进化粒子群蚁群蜂群布谷鸟萤火虫蝙蝠灰狼鲸鱼花朵和声变邻域局部全
局随机确定启发元启发超启发混合自适应多目标约束无约束连续离散整数线
性非线性动态贪心分治回溯分支广度深度迭代双向最佳一致递归简化山峰模
拟禁忌遗传差分粒子蚁群蜂群布谷萤火蝙蝠灰狼鲸鱼花朵和声变邻局部全局
""".replace('\n', '')

# Remove duplicates while preserving order
def unique_chars(text):
    seen = set()
    result = []
    for char in text:
        if char not in seen and not char.isspace():
            seen.add(char)
            result.append(char)
    return ''.join(result)

COMMON_CJK_CHARS = unique_chars(COMMON_CJK_CHARS)


def find_cjk_font():
    """Try to find a suitable CJK font on the system."""
    # Common font paths on different systems
    font_candidates = [
        # macOS
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/Library/Fonts/Microsoft/SimHei.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
        # Linux
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        # Windows (if running through WSL or similar)
        "/mnt/c/Windows/Fonts/simhei.ttf",
        "/mnt/c/Windows/Fonts/msyh.ttc",
    ]

    for font_path in font_candidates:
        if os.path.exists(font_path):
            return font_path

    return None


def migrate_texture(input_path, output_path, font_path=None, chars_path=None,
                    skip_cjk=False, preview=False):
    """
    Migrate 2048x2048 texture to 4096x4096 and add CJK characters.
    """
    print(f"Loading input texture: {input_path}")

    if not os.path.exists(input_path):
        print(f"Error: Input file not found: {input_path}")
        return False

    # Load input texture
    old_img = Image.open(input_path).convert('RGBA')
    old_w, old_h = old_img.size
    print(f"Input texture size: {old_w}x{old_h}")

    if old_w != 2048 or old_h != 2048:
        print(f"Warning: Expected 2048x2048, got {old_w}x{old_h}")

    # Create new 4096x4096 texture (transparent background)
    new_img = Image.new('RGBA', (4096, 4096), (0, 0, 0, 0))

    # ===== STEP 1: Copy Sprite region =====
    # Old: rows 0-1535, 6 block rows × 8 block cols = 48 blocks
    # New: rows 0-2047, 8 block rows × 16 block cols = 128 blocks
    # Each block is 256×256 pixels, 16×16 sprites of 16×16 pixels each
    print("Copying Sprite region...")

    for old_block_y in range(6):  # 6 rows of blocks in old texture
        for old_block_x in range(8):  # 8 cols of blocks in old texture
            old_block_idx = old_block_y * 8 + old_block_x

            # Calculate old pixel coordinates
            old_px = old_block_x * 256
            old_py = old_block_y * 256

            # In new layout, blocks are arranged in 16 columns
            new_block_x = old_block_idx % 16
            new_block_y = old_block_idx // 16

            # Calculate new pixel coordinates
            new_px = new_block_x * 256
            new_py = new_block_y * 256

            # Copy block
            block = old_img.crop((old_px, old_py, old_px + 256, old_py + 256))
            new_img.paste(block, (new_px, new_py))

    print(f"  Copied 48 sprite blocks (12,288 sprites)")

    # ===== STEP 2: Copy TUI region =====
    # Old: rows 1536-2047, blocks 48-52 (5 blocks), each 256×512 pixels (16×16 chars at 16×32 each)
    # New: rows 2560-3071 (512px), blocks 160-169 (10 blocks), each 256×512 pixels
    print("Copying TUI region...")

    # In old texture, TUI is at y=1536, blocks 0-4 (cols 0-79 in 16px units, or 0-1279 pixels)
    # Each old TUI block: 256px wide × 512px tall, 16 cols × 16 rows of 16×32 chars = 256 chars
    # In new texture, TUI starts at y=2560, same block format (256×512 per block)

    # Copy entire TUI region (1280×512 = 5 blocks × 256 wide × 512 tall)
    tui_width = 5 * 256  # 1280 pixels
    tui_height = 512
    tui_region = old_img.crop((0, 1536, tui_width, 1536 + tui_height))

    # Place at new TUI region start (y=2560, x=0)
    new_img.paste(tui_region, (0, 2560))

    print(f"  Copied TUI region (1280 chars)")

    # ===== STEP 3: Copy Emoji region =====
    # Old: rows 1536-2047, blocks 53-55 (3 blocks at cols 80-127 in 16px units)
    # New: rows 2560-3071 (512px), blocks 170-175, starting at x=2560
    print("Copying Emoji region...")

    # Old Emoji: starts at x=1280 (80*16), y=1536, width=768 (48*16), height=512
    # Each emoji is 32×32, so old has 24 cols × 16 rows = 384 emoji
    old_emoji_x = 1280
    old_emoji_y = 1536
    old_emoji_w = 768
    old_emoji_h = 512

    emoji_region = old_img.crop((old_emoji_x, old_emoji_y,
                                  old_emoji_x + old_emoji_w,
                                  old_emoji_y + old_emoji_h))

    # Place at new Emoji region (y=2560, x=2560)
    new_img.paste(emoji_region, (2560, 2560))

    print(f"  Copied Emoji region (384 emoji)")

    # ===== STEP 4: Render CJK characters =====
    if not skip_cjk:
        print("Rendering CJK characters...")

        # Find font
        if font_path is None:
            font_path = find_cjk_font()

        if font_path is None:
            print("  Warning: No CJK font found. Skipping CJK rendering.")
            print("  Please specify a font with --font option.")
        else:
            print(f"  Using font: {font_path}")

            # Load characters
            if chars_path and os.path.exists(chars_path):
                with open(chars_path, 'r', encoding='utf-8') as f:
                    cjk_chars = unique_chars(f.read())
                print(f"  Loaded {len(cjk_chars)} characters from {chars_path}")
            else:
                cjk_chars = COMMON_CJK_CHARS
                print(f"  Using built-in {len(cjk_chars)} common CJK characters")

            # Limit to 4096 characters (CJK region capacity)
            max_cjk = 4096
            if len(cjk_chars) > max_cjk:
                cjk_chars = cjk_chars[:max_cjk]
                print(f"  Truncated to {max_cjk} characters (region capacity)")

            # Load font
            try:
                font = ImageFont.truetype(font_path, 28)  # 28pt for 32×32 cell with padding
            except Exception as e:
                print(f"  Error loading font: {e}")
                font = None

            if font:
                draw = ImageDraw.Draw(new_img)

                # CJK region: y=3072, 128 cols × 32 rows, 32×32 each
                cjk_start_y = 3072
                cjk_cols = 128
                cjk_rows = 32
                char_size = 32

                for i, char in enumerate(cjk_chars):
                    col = i % cjk_cols
                    row = i // cjk_cols

                    if row >= cjk_rows:
                        break

                    x = col * char_size
                    y = cjk_start_y + row * char_size

                    # Get text bounding box for centering
                    try:
                        bbox = font.getbbox(char)
                        tw = bbox[2] - bbox[0]
                        th = bbox[3] - bbox[1]

                        # Center the character in the cell
                        tx = x + (char_size - tw) // 2 - bbox[0]
                        ty = y + (char_size - th) // 2 - bbox[1]

                        # Draw white character (color will be applied via shader)
                        draw.text((tx, ty), char, font=font, fill=(255, 255, 255, 255))
                    except Exception as e:
                        # Skip characters that can't be rendered
                        pass

                print(f"  Rendered {min(len(cjk_chars), cjk_cols * cjk_rows)} CJK characters")

                # Generate CJK mapping file
                map_path = output_path.replace('.png', '_cjk_map.json')
                generate_cjk_map(cjk_chars, cjk_start_y, cjk_cols, char_size, map_path)

    # ===== STEP 5: Save output =====
    print(f"Saving output texture: {output_path}")
    new_img.save(output_path, 'PNG')

    # ===== STEP 6: Generate preview (optional) =====
    if preview:
        preview_path = output_path.replace('.png', '_preview.png')
        generate_preview(new_img, preview_path)

    print("Done!")
    return True


def generate_cjk_map(chars, start_y, cols, char_size, output_path):
    """Generate JSON mapping file for CJK characters."""
    import json

    mapping = {
        "version": 1,
        "char_size": [char_size, char_size],
        "region_start_y": start_y,
        "cols": cols,
        "chars": {}
    }

    for i, char in enumerate(chars):
        col = i % cols
        row = i // cols
        x = col * char_size
        y = start_y + row * char_size
        mapping["chars"][char] = {"x": x, "y": y, "idx": i}

    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(mapping, f, ensure_ascii=False, indent=2)

    print(f"  Generated CJK mapping: {output_path}")


def generate_preview(img, output_path):
    """Generate a preview image with region labels."""
    from PIL import ImageDraw

    preview = img.copy()
    draw = ImageDraw.Draw(preview)

    # Draw region boundaries (10 Sprite Rows layout)
    regions = [
        (0, 0, 4096, 2560, "Sprite (40,960)", (255, 0, 0, 128)),
        (0, 2560, 2560, 3072, "TUI (2,560)", (0, 255, 0, 128)),
        (2560, 2560, 4096, 3072, "Emoji (768)", (0, 0, 255, 128)),
        (0, 3072, 4096, 4096, "CJK (4,096)", (255, 255, 0, 128)),
    ]

    for x1, y1, x2, y2, label, color in regions:
        # Draw semi-transparent overlay
        overlay = Image.new('RGBA', (x2-x1, y2-y1), color)
        preview.paste(overlay, (x1, y1), overlay)

        # Draw border
        draw.rectangle([x1, y1, x2-1, y2-1], outline=(255, 255, 255, 255), width=2)

        # Draw label
        draw.text((x1 + 10, y1 + 10), label, fill=(255, 255, 255, 255))

    # Scale down for preview
    preview_small = preview.resize((1024, 1024), Image.Resampling.LANCZOS)
    preview_small.save(output_path, 'PNG')
    print(f"  Generated preview: {output_path}")


def main():
    parser = argparse.ArgumentParser(
        description='Migrate symbols.png from 2048x2048 to 4096x4096 layout'
    )
    parser.add_argument('--input', '-i', default='assets/pix/symbols.png',
                        help='Input 2048x2048 texture path')
    parser.add_argument('--output', '-o', default='assets/pix/symbols_4096.png',
                        help='Output 4096x4096 texture path')
    parser.add_argument('--font', '-f', default=None,
                        help='TTF font for CJK rendering')
    parser.add_argument('--chars', '-c', default=None,
                        help='File with CJK characters to render')
    parser.add_argument('--no-cjk', action='store_true',
                        help='Skip CJK character rendering')
    parser.add_argument('--preview', '-p', action='store_true',
                        help='Generate preview image')

    args = parser.parse_args()

    # Resolve paths relative to script location
    script_dir = Path(__file__).parent.parent
    input_path = script_dir / args.input
    output_path = script_dir / args.output

    success = migrate_texture(
        str(input_path),
        str(output_path),
        font_path=args.font,
        chars_path=args.chars,
        skip_cjk=args.no_cjk,
        preview=args.preview
    )

    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()
