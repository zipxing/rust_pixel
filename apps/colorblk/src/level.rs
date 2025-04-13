use colorblk_lib::{Block, ColorBlkStage, Gate, Obstacle, SHAPE_IDX, SHAPE_IDX_COCOS, SHAPE};
use log::info;
use serde_json::{from_str, Value};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub const COLORBLKW: u16 = 100;
pub const COLORBLKH: u16 = 61;
pub const CELLW: usize = 10;
pub const CELLH: usize = 5;

/// 从JSON文件加载关卡数据
pub fn load_level_from_json(filename: &str) -> ColorBlkStage {
    // 尝试打开文件
    let file_path = Path::new(filename);
    info!("尝试加载关卡文件: {}", file_path.display());

    let mut file = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            // 尝试从根目录加载
            let root_path = Path::new(".").join(filename);
            info!("尝试从根目录加载关卡文件: {}", root_path.display());

            match File::open(&root_path) {
                Ok(file) => file,
                Err(e2) => {
                    // 文件加载失败时使用默认值
                    info!(
                        "无法打开关卡文件 {} 或 {}: {}, {}",
                        file_path.display(),
                        root_path.display(),
                        e,
                        e2
                    );
                    let mut stage = ColorBlkStage::new(5, 9);
                    stage.blocks = create_default_blocks();
                    stage.gates = create_default_gates(&stage);
                    stage.obstacles = create_default_obstacles();
                    return stage;
                }
            }
        }
    };

    // 读取文件内容
    let mut json_data = String::new();
    if let Err(e) = file.read_to_string(&mut json_data) {
        info!("无法读取关卡文件内容: {}", e);
        let mut stage = ColorBlkStage::new(5, 9);
        stage.blocks = create_default_blocks();
        stage.gates = create_default_gates(&stage);
        stage.obstacles = create_default_obstacles();
        return stage;
    }

    // 解析JSON
    let json_value: Value = match from_str(&json_data) {
        Ok(value) => value,
        Err(e) => {
            info!("无法解析JSON: {}", e);
            let mut stage = ColorBlkStage::new(5, 9);
            stage.blocks = create_default_blocks();
            stage.gates = create_default_gates(&stage);
            stage.obstacles = create_default_obstacles();
            return stage;
        }
    };

    // 从JSON中获取宽度和高度
    let width = json_value["wh"].as_u64().unwrap_or(5) as usize - 2;
    let height = json_value["ht"].as_u64().unwrap_or(9) as usize - 2;

    info!("成功加载关卡，大小: {}x{}", width, height);

    // 创建一个新关卡
    let mut blocks = Vec::new();
    let mut gates = Vec::new();
    let mut obstacles = Vec::new();

    // 用于ID自增
    let mut next_id: u8 = 1;

    // 创建原始ID到新ID的映射表
    let mut id_map: HashMap<u32, u8> = HashMap::new();
    let mut original_links: HashMap<u8, Vec<u8>> = HashMap::new();

    // 解析所有槽位
    if let Some(slots) = json_value["ss"].as_array() {
        info!("发现 {} 个槽位", slots.len());

        // 第一轮：创建ID映射
        for slot in slots.iter() {
            let block_type = slot["bp"].as_u64().unwrap_or(0) as u8;

            // 只处理普通方块(type=1)
            if block_type == 1 {
                let raw_id = slot["bd"].as_u64().unwrap_or(0) as u32;
                // 创建新ID并建立映射关系
                if raw_id > 0 {
                    let id = next_id;
                    next_id += 1;
                    id_map.insert(raw_id, id);
                } else {
                    // 没有原始ID的方块也分配一个新ID
                    next_id += 1;
                }
            }
        }

        // 第二轮：创建方块并处理链接
        for (_slot_index, slot) in slots.iter().enumerate() {
            // 获取基本属性
            let mut x = slot["x"].as_u64().unwrap_or(0) as u8;
            let y = slot["y"].as_u64().unwrap_or(0) as u8;
            let block_type = slot["bp"].as_u64().unwrap_or(0) as u8;

            // 从bd中解析id
            let raw_id = slot["bd"].as_u64().unwrap_or(0) as u32;
            let block_id = if raw_id > 0 {
                *id_map.get(&raw_id).unwrap_or(&next_id)
            } else {
                let id = next_id;
                next_id += 1;
                id
            };

            let block_shape_id = slot["bi"].as_u64().unwrap_or(0) as u8;

            // info!(
            //     "处理槽位 #{}: 位置({},{}), 类型={}, 原始ID={}, 新ID={}",
            //     slot_index, x, y, block_type, raw_id, block_id
            // );

            // 根据方块类型处理不同的对象
            match block_type {
                1 => {
                    let mut shape_idx = block_shape_id;

                    // 适配js定义中锚点差异
                    match shape_idx {
                        12 | 17 | 22 | 25 | 26 | 34 => {
                            x -= 1;
                        }
                        16 => x -= 2,
                        33 => x -= 3,
                        _ => {}
                    }

                    shape_idx = SHAPE_IDX_COCOS[block_shape_id as usize] as u8;

                    // 获取颜色
                    let mut color = 0;
                    if let Some(layers) = slot["l"].as_array() {
                        if !layers.is_empty() {
                            if let Some(first_layer) = layers.first() {
                                if first_layer.is_object() && first_layer["b"].is_u64() {
                                    color = first_layer["b"].as_u64().unwrap_or(0) as u8;
                                }
                            }
                        }
                    };

                    // 获取其他属性
                    let block_limit_dir = slot["br"].as_u64().unwrap_or(0) as u8;
                    let ice_count = slot["i"].as_u64().unwrap_or(0) as u8;
                    let key = slot["k"].as_u64().unwrap_or(0) as u8;
                    let lock = slot["lt"].as_u64().unwrap_or(0) as u8;
                    let star = slot["de"].as_u64().unwrap_or(0) as u8;
                    let scissor = slot["h"].as_u64().unwrap_or(0) as u8;

                    // 处理链接 - 获取ls字段中的直接连接
                    let mut direct_links = Vec::new();
                    if let Some(links) = slot["ls"].as_array() {
                        for link_data in links {
                            if link_data.is_object() {
                                if let Some(linked_raw_id) = link_data["t"].as_u64() {
                                    let linked_raw_id = linked_raw_id as u32;
                                    // 转换为新ID
                                    if let Some(&linked_new_id) = id_map.get(&linked_raw_id) {
                                        direct_links.push(linked_new_id);
                                        // info!("links.........{:?}", direct_links);
                                    }
                                }
                            }
                        }
                    }

                    // 保存直接链接关系，供后续合并使用
                    if !direct_links.is_empty() {
                        original_links.insert(block_id, direct_links.clone());
                    }

                    // 处理绳索
                    let mut ropes = Vec::new();
                    if let Some(rope_info) = slot["r"].as_array() {
                        for rope in rope_info {
                            if let Some(rope_color) = rope["c"].as_u64() {
                                ropes.push(rope_color as u8);
                            }
                        }
                    }

                    // 创建方块，暂时保留空的link字段，后续会更新
                    let block = Block {
                        id: block_id,
                        shape: shape_idx,
                        color,
                        color2: 0, // JSON中可能没有color2属性，暂时默认为0
                        star,
                        dir: block_limit_dir,
                        ice: ice_count,
                        key,
                        lock,
                        scissor,
                        ropes,
                        x: x - 1,
                        y: y - 1,
                        link: Vec::new(), // 暂时为空，稍后再填充
                    };

                    // info!("添加移动方块: ID={}, 颜色={}, 形状={}, 直接链接={:?}",
                    //       block_id, color, block_shape_id, direct_links);
                    blocks.push(block);
                }
                2 => {
                    // 过滤掉墙
                    if x == 0 || y == 0 || x == width as u8 + 1 || y == height as u8 + 1 {
                        continue;
                    }
                    // 墙壁或障碍物
                    // info!("跳过墙壁障碍物: ({},{})", x, y);
                    let mut shape_idx = block_shape_id;
                    
                    // 适配js定义中锚点差异，与方块处理类似
                    match shape_idx {
                        12 | 17 | 22 | 25 | 26 | 34 => {
                            x -= 1;
                        }
                        16 => x -= 2,
                        33 => x -= 3,
                        _ => {}
                    }
                    
                    // 获取实际形状
                    if shape_idx < SHAPE_IDX_COCOS.len() as u8 {
                        shape_idx = SHAPE_IDX_COCOS[block_shape_id as usize] as u8;
                        
                        // 获取障碍物颜色（允许通过的颜色）
                        let mut allow_color = 255;
                        if let Some(layers) = slot["l"].as_array() {
                            if !layers.is_empty() {
                                if let Some(first_layer) = layers.first() {
                                    if first_layer.is_object() && first_layer["b"].is_u64() {
                                        allow_color = first_layer["b"].as_u64().unwrap_or(0) as u8;
                                    }
                                }
                            }
                        };
                        
                        // 根据形状创建多个障碍物
                        let shape_data = &SHAPE[shape_idx as usize];
                        for grid_y in 0..5 {
                            for grid_x in 0..5 {
                                if shape_data.grid[grid_y][grid_x] == 1 {
                                    // 计算棋盘上的实际坐标
                                    let board_x = (x - 1) as usize + (grid_x - shape_data.rect.x);
                                    let board_y = (y - 1) as usize + (grid_y - shape_data.rect.y);
                                    
                                    obstacles.push(Obstacle {
                                        x: board_x as u8,
                                        y: board_y as u8,
                                        allow_color,
                                    });
                                }
                            }
                        }
                        
                        info!("添加障碍物: 位置({},{}), 允许颜色={}, 形状ID={}", 
                             x - 1, y - 1, allow_color, shape_idx);
                    }
                }
                3 => {
                    // 门
                    let color = if let Some(layers) = slot["l"].as_array() {
                        if !layers.is_empty() {
                            if let Some(first_layer) = layers.first() {
                                if first_layer.is_object() && first_layer["b"].is_u64() {
                                    first_layer["b"].as_u64().unwrap_or(0) as u8
                                } else {
                                    0
                                }
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    let door_dir = slot["dr"].as_u64().unwrap_or(0) as u8;
                    let star = slot["de"].as_u64().unwrap_or(0) as u8;
                    let ice = slot["i"].as_u64().unwrap_or(0) as u8;
                    let lock = slot["m"].as_u64().unwrap_or(0) as u8;
                    let door_shape_id = slot["bi"].as_u64().unwrap_or(0) as u8;

                    // 根据门的方向创建不同的门
                    let (width, height) = match door_dir {
                        // 左右门
                        0 | 1 => match door_shape_id {
                            0 => (0, 1),
                            1 => (0, 2),
                            3 => (0, 3),
                            _ => (0, 1),
                        },
                        // 上下门
                        2 | 3 => match door_shape_id {
                            0 => (1, 0),
                            2 => (2, 0),
                            4 => (3, 0),
                            _ => (1, 0),
                        },
                        _ => (1, 0), // 默认为上/下门
                    };

                    let mut gate = Gate {
                        x: if x == 0 { 0 } else { x - 1 },
                        y: if y == 0 { 0 } else { y - 1 },
                        color,
                        ice,
                        lock,
                        star,
                        width,
                        height,
                        switch: true, // 默认开启状态
                    };

                    match door_dir {
                        0 => gate.x -= 1,
                        2 => gate.y -= 1,
                        _ => {}
                    }

                    // info!(
                    //     "添加门: 位置({},{}), 颜色={}, 方向={}",
                    //     x, y, color, door_dir
                    // );
                    gates.push(gate);
                }
                4 => {
                    // 普通障碍物
                    let mut allow_color = 0;
                    if let Some(layers) = slot["l"].as_array() {
                        if !layers.is_empty() {
                            if let Some(first_layer) = layers.first() {
                                if first_layer.is_object() && first_layer["b"].is_u64() {
                                    allow_color = first_layer["b"].as_u64().unwrap_or(0) as u8;
                                }
                            }
                        }
                    };

                    let obstacle = Obstacle { x, y, allow_color };

                    // info!("添加障碍物: 位置({},{}), 允许颜色={}", x, y, allow_color);
                    obstacles.push(obstacle);
                }
                _ => {
                    info!("跳过未知类型 {} 位置({},{})", block_type, x, y);
                }
            }
        }
    }

    // 构建无向图表示块之间的连接关系
    let mut connections: HashMap<u8, Vec<u8>> = HashMap::new();

    // 初始化连接图
    for block in &blocks {
        connections.insert(block.id, Vec::new());
    }

    // 添加所有直接连接关系（确保双向）
    for (id, links) in &original_links {
        for &linked_id in links {
            // 添加双向连接
            connections
                .entry(*id)
                .or_insert_with(Vec::new)
                .push(linked_id);
            connections
                .entry(linked_id)
                .or_insert_with(Vec::new)
                .push(*id);
        }
    }

    // 去重连接
    for (_, links) in connections.iter_mut() {
        links.sort();
        links.dedup();
    }

    // 查找连通分量（找出所有相互连接的组）
    let mut visited: HashSet<u8> = HashSet::new();
    let mut groups: Vec<Vec<u8>> = Vec::new();

    // 深度优先搜索找出所有连接的块
    fn dfs(
        node: u8,
        visited: &mut HashSet<u8>,
        connections: &HashMap<u8, Vec<u8>>,
        group: &mut Vec<u8>,
    ) {
        visited.insert(node);
        group.push(node);

        if let Some(neighbors) = connections.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    dfs(neighbor, visited, connections, group);
                }
            }
        }
    }

    // 为每个未访问的块启动DFS
    for &block_id in connections.keys() {
        if !visited.contains(&block_id) {
            let mut current_group = Vec::new();
            dfs(block_id, &mut visited, &connections, &mut current_group);
            if !current_group.is_empty() {
                current_group.sort(); // 排序使结果更一致
                groups.push(current_group);
            }
        }
    }

    // 将组分配给块
    for block in &mut blocks {
        // 找到包含当前块的组
        for group in &groups {
            if group.contains(&block.id) {
                // 链接到组中除自己以外的所有块
                let links: Vec<u8> = group.iter().cloned().collect();

                if links.len() == 1 {
                    block.link = vec![];
                } else {
                    block.link = links;
                }
                // info!("方块 ID={} 的最终组连接: {:?}", block.id, block.link);
                break;
            }
        }
    }

    // // 输出所有识别的组
    // info!("\n识别出 {} 个组:", groups.len());
    // for (i, group) in groups.iter().enumerate() {
    //     info!("组 #{}: {:?}", i + 1, group);
    // }

    // 在返回LevelData前打印解析出的数据
    info!("\n========== 解析结果 ==========");
    info!(
        "总计解析: {} 个方块, {} 个门, {} 个障碍物",
        blocks.len(),
        gates.len(),
        obstacles.len()
    );

    info!("\n---------- 方块 ----------");
    for (i, block) in blocks.iter().enumerate() {
        info!("方块 #{}: ID={}, 位置=({},{}), 形状={}, 颜色={}, 冰层={}, 钥匙={}, 锁={}, 星标={}, 链接={:?}",
            i, block.id, block.x, block.y, block.shape, block.color, block.ice, block.key, block.lock, block.star, block.link);
    }

    info!("\n---------- 门 ----------");
    for (i, gate) in gates.iter().enumerate() {
        info!(
            "门 #{}: 位置=({},{}), 颜色={}, 宽度={}, 高度={}, 冰层={}, 锁={}, 星标={}, 开关={}",
            i,
            gate.x,
            gate.y,
            gate.color,
            gate.width,
            gate.height,
            gate.ice,
            gate.lock,
            gate.star,
            gate.switch
        );
    }

    info!("\n---------- 障碍物 ----------");
    for (i, obstacle) in obstacles.iter().enumerate() {
        info!(
            "障碍物 #{}: 位置=({},{}), 允许颜色={}",
            i, obstacle.x, obstacle.y, obstacle.allow_color
        );
    }
    info!("============================\n");

    let mut stage = ColorBlkStage::new(width, height);
    stage.blocks = blocks;
    stage.gates = gates;
    stage.obstacles = obstacles;
    stage
}

/// 创建默认障碍
fn create_default_obstacles() -> Vec<Obstacle> {
    vec![
        Obstacle {
            x: 2,
            y: 3,
            allow_color: 3,
        },
        Obstacle {
            x: 2,
            y: 4,
            allow_color: 3,
        },
        Obstacle {
            x: 2,
            y: 5,
            allow_color: 3,
        },
    ]
}

/// 创建默认的门
fn create_default_gates(stage: &ColorBlkStage) -> Vec<Gate> {
    ////-------双色块---------
    //vec![
    //    // 上方门(1色)
    //    Gate {
    //        x: 0,
    //        y: 0,
    //        color: 1,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 2,
    //        height: 0,
    //        switch: true, // 默认开启状态
    //    },
    //    // 下方门(2色)
    //    Gate {
    //        x: 0,
    //        y: (stage.board_height - 1) as u8,
    //        color: 2,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 2,
    //        height: 0,
    //        switch: true, // 默认开启状态
    //    },
    //]
    //// 测试结束

    //-------星门&半透障碍测试---------
    vec![
        // 上方门(1色)
        Gate {
            x: 0,
            y: 0,
            color: 1,
            ice: 0,
            lock: 0,
            star: 0,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 上方门(2色)
        Gate {
            x: 2,
            y: 0,
            color: 2,
            ice: 0,
            lock: 0,
            star: 1,
            width: 1,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 上方门(3色)
        Gate {
            x: 3,
            y: 0,
            color: 3,
            ice: 0,
            lock: 0,
            star: 1,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 下方门(4色)
        Gate {
            x: 0,
            y: (stage.board_height - 1) as u8,
            color: 4,
            ice: 0,
            lock: 0,
            star: 0,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 下方门(5色)
        Gate {
            x: 2,
            y: (stage.board_height - 1) as u8,
            color: 5,
            ice: 0,
            lock: 0,
            star: 1,
            width: 1,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 下方门(6色)
        Gate {
            x: 3,
            y: (stage.board_height - 1) as u8,
            color: 6,
            ice: 0,
            lock: 0,
            star: 0,
            width: 2,
            height: 0,
            switch: true, // 默认开启状态
        },
        // 左方门(5色)
        Gate {
            x: 0,
            y: 2,
            color: 5,
            ice: 0,
            lock: 0,
            star: 0,
            width: 0,
            height: 2,
            switch: true, // 默认开启状态
        },
        // 左方门(7色)
        Gate {
            x: 0,
            y: 4,
            color: 7,
            ice: 0,
            lock: 0,
            star: 0,
            width: 0,
            height: 3,
            switch: true, // 默认开启状态
        },
        // 右方门(2色)
        Gate {
            x: (stage.board_width - 1) as u8,
            y: 2,
            color: 2,
            ice: 0,
            lock: 0,
            star: 0,
            width: 0,
            height: 2,
            switch: true, // 默认开启状态
        },
        // 右方门(8色)
        Gate {
            x: (stage.board_width - 1) as u8,
            y: 4,
            color: 8,
            ice: 0,
            lock: 0,
            star: 0,
            width: 0,
            height: 3,
            switch: true, // 默认开启状态
        },
    ]
    //-------------测试结束----------------

    ////-------组合块测试---------
    //vec![
    //    // 上方门(1色)
    //    Gate {
    //        x: 2,
    //        y: 0,
    //        color: 1,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 1,
    //        height: 0,
    //        switch: true, // 默认开启状态
    //    },
    //    // 下方门(2色)
    //    Gate {
    //        x: 2,
    //        y: (stage.board_height - 1) as u8,
    //        color: 2,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 1,
    //        height: 0,
    //        switch: true, // 默认开启状态
    //    },
    //    // 左方门(3色)
    //    Gate {
    //        x: 0,
    //        y: 0,
    //        color: 3,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 0,
    //        height: 1,
    //        switch: true, // 默认开启状态
    //    },
    //    // 左方门(4色)
    //    Gate {
    //        x: 0,
    //        y: 3,
    //        color: 4,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 0,
    //        height: 2,
    //        switch: true, // 默认开启状态
    //    },
    //    // 右方门(5色)
    //    Gate {
    //        x: (stage.board_width - 1) as u8,
    //        y: 3,
    //        color: 5,
    //        ice: 0,
    //        lock: 0,
    //        star: 0,
    //        width: 0,
    //        height: 2,
    //        switch: true, // 默认开启状态
    //    },
    //]
    ////-------------组合块测试结束----------------

    //-----普通关卡测试-----
    // vec![
    //     // 上方门(1色)
    //     Gate {
    //         x: 0,
    //         y: 0,
    //         color: 1,
    //         width: 3,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 上方门(2色)
    //     Gate {
    //         x: 3,
    //         y: 0,
    //         color: 2,
    //         width: 3,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 下方门(3色)
    //     Gate {
    //         x: 0,
    //         y: (stage.board_height - 1) as u8,
    //         color: 3,
    //         width: 2,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 下方门(4色)
    //     Gate {
    //         x: 4,
    //         y: (stage.board_height - 1) as u8,
    //         color: 4,
    //         width: 2,
    //         height: 0,
    //         switch: true, // 默认开启状态
    //     },
    //     // 左方门(5色)
    //     Gate {
    //         x: 0,
    //         y: 2,
    //         color: 5,
    //         width: 0,
    //         height: 2,
    //         switch: true, // 默认开启状态
    //     },
    //     // 右方门(6色)
    //     Gate {
    //         x: (stage.board_width - 1) as u8,
    //         y: 2,
    //         color: 6,
    //         width: 0,
    //         height: 2,
    //         switch: true, // 默认开启状态
    //     },
    // ]
    //-----普通关卡测试结束-----
}

fn create_default_blocks() -> Vec<Block> {
    // vec![
    //     Block {
    //         id: 1,
    //         shape: SHAPE_IDX[0] as u8,
    //         color: 1,
    //         color2: 2,
    //         star: 0,
    //         dir: 0,
    //         ropes: vec![],
    //         scissor: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 0,
    //         y: 3,
    //         link: Vec::new(),
    //     },
    // ]
    //-----星门半透明测试-----
    vec![
        Block {
            id: 1,
            shape: SHAPE_IDX[1] as u8,
            color: 4,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 0,
            link: Vec::new(),
        },
        Block {
            id: 2,
            shape: SHAPE_IDX[4] as u8,
            color: 8,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 2,
            y: 0,
            link: Vec::new(),
        },
        Block {
            id: 3,
            shape: SHAPE_IDX[1] as u8,
            color: 7,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 0,
            link: Vec::new(),
        },
        Block {
            id: 4,
            shape: SHAPE_IDX[2] as u8, // 纵向两个方块
            color: 2,                  // 绿色，对应右方门
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 1,
            y: 1,
            link: Vec::new(),
        },
        Block {
            id: 5,
            shape: SHAPE_IDX[2] as u8,
            color: 6,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 1,
            link: Vec::new(),
        },
        Block {
            id: 6,
            shape: SHAPE_IDX[2] as u8,
            color: 5,
            color2: 0,
            star: 1,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 2,
            link: Vec::new(),
        },
        Block {
            id: 7,
            shape: SHAPE_IDX[2] as u8,
            color: 2,
            color2: 0,
            star: 1,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 4,
            y: 2,
            link: Vec::new(),
        },
        Block {
            id: 8,
            shape: SHAPE_IDX[1] as u8,
            color: 6,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 4,
            link: Vec::new(),
        },
        Block {
            id: 9,
            shape: SHAPE_IDX[1] as u8,
            color: 3,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 4,
            link: Vec::new(),
        },
        Block {
            id: 10,
            shape: SHAPE_IDX[2] as u8,
            color: 8,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 5,
            link: Vec::new(),
        },
        Block {
            id: 11,
            shape: SHAPE_IDX[2] as u8,
            color: 1,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 1,
            y: 5,
            link: Vec::new(),
        },
        Block {
            id: 12,
            shape: SHAPE_IDX[2] as u8,
            color: 4,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 5,
            link: Vec::new(),
        },
        Block {
            id: 13,
            shape: SHAPE_IDX[2] as u8,
            color: 7,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 4,
            y: 5,
            link: Vec::new(),
        },
        Block {
            id: 14,
            shape: SHAPE_IDX[9] as u8,
            color: 3,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 0,
            y: 7,
            link: Vec::new(),
        },
        Block {
            id: 15,
            shape: SHAPE_IDX[4] as u8,
            color: 3,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 2,
            y: 6,
            link: Vec::new(),
        },
        Block {
            id: 16,
            shape: SHAPE_IDX[9] as u8,
            color: 1,
            color2: 0,
            star: 0,
            dir: 0,
            ropes: vec![],
            scissor: 0,
            ice: 0,
            key: 0,
            lock: 0,
            x: 3,
            y: 7,
            link: Vec::new(),
        },
    ]
    //-----测试结束-----

    ////-----组合块测试-----
    //vec![
    //    Block {
    //        id: 1,
    //        shape: SHAPE_IDX[0] as u8, // 单个方块
    //        color: 2,                  // 红色，对应上方门
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 1,
    //        y: 0,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 2,
    //        shape: SHAPE_IDX[9] as u8, // 横向两个方块
    //        color: 4,                  // 蓝色，对应下方门
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 2,
    //        y: 0,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 3,
    //        shape: SHAPE_IDX[2] as u8, // 纵向两个方块
    //        color: 3,                  // 绿色，对应右方门
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 0,
    //        y: 1,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 4,
    //        shape: SHAPE_IDX[0] as u8, // 纵向两个方块
    //        color: 2,                  // 绿色，对应右方门
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 3,
    //        y: 2,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 5,
    //        shape: SHAPE_IDX[0] as u8,
    //        color: 3,
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 0,
    //        y: 3,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 6,
    //        shape: SHAPE_IDX[1] as u8,
    //        color: 5,
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 1,
    //        y: 3,
    //        // link: vec![1, 2],
    //        link: vec![6, 8],
    //        // link: Vec::new(),
    //    },
    //    Block {
    //        id: 7,
    //        shape: SHAPE_IDX[2] as u8,
    //        color: 4,
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 3,
    //        y: 3,
    //        link: Vec::new(),
    //    },
    //    Block {
    //        id: 8,
    //        shape: SHAPE_IDX[0] as u8,
    //        color: 1,
    //        color2: 0,
    //        star: 0,
    //        dir: 0,
    //        ropes: vec![],
    //        scissor: 0,
    //        ice: 0,
    //        key: 0,
    //        lock: 0,
    //        x: 2,
    //        y: 4,
    //        // link: Vec::new(),
    //        link: vec![6, 8],
    //    },
    //]
    ////-----组合块测试结束-----

    // vec![
    //     Block {
    //         id: 1,
    //         shape: SHAPE_IDX[3] as u8, // 单个方块
    //         color: 2,                  // 红色，对应上方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 0,
    //         y: 1,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 2,
    //         shape: SHAPE_IDX[3] as u8, // 横向两个方块
    //         color: 1,                  // 蓝色，对应下方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 3,
    //         y: 1,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 3,
    //         shape: SHAPE_IDX[2] as u8, // 纵向两个方块
    //         color: 4,                  // 绿色，对应右方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 0,
    //         y: 2,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 4,
    //         shape: SHAPE_IDX[3] as u8, // 纵向两个方块
    //         color: 6,                  // 绿色，对应右方门
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 2,
    //         y: 2,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 5,
    //         shape: SHAPE_IDX[2] as u8,
    //         color: 3,
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 5,
    //         y: 2,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 6,
    //         shape: SHAPE_IDX[3] as u8,
    //         color: 5,
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 1,
    //         y: 3,
    //         // link: vec![6, 8],
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 7,
    //         shape: SHAPE_IDX[5] as u8,
    //         color: 3,
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 1,
    //         y: 4,
    //         link: Vec::new(),
    //     },
    //     Block {
    //         id: 8,
    //         shape: SHAPE_IDX[7] as u8,
    //         color: 4,
    //         color2: 0,
    //         ice: 0,
    //         key: 0,
    //         lock: 0,
    //         x: 3,
    //         y: 4,
    //         link: Vec::new(),
    //         // link: vec![6, 8],
    //     },
    // ]
}
