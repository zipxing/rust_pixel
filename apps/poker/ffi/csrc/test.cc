#include <stdio.h>
#include "poker_ffi.h"

void test_poker_card() {
    rs_PokerCards *a = rs_PokerCards_new();
    printf("new pokercards address = %p\n", a);

    unsigned short hand[] = {1, 2, 3, 4, 14, 15};
    char r = rs_PokerCards_assign(a, hand, 6);
    printf("assign pokercards ret = %d\n", r);

    rs_Counter *c2 = rs_PokerCards_get_counter(a, rs_Suit::spade);
    printf("spade counter.n = %d\n", c2->n);

    rs_CardBuffer cb = rs_PokerCards_get_cards(a);
    printf("len = %ld\n", cb.len);
    for(int i=0; i<cb.len; i++) {
        printf("  card %d = (%d, %d)\n", hand[i], cb.data[i].suit, cb.data[i].number);
    }

    rs_Counter *c = rs_Counter_new(rs_Suit::spade);
    printf("new counter.n = %d\n", c->n);
    rs_Counter_free(c);

    rs_CardBuffer_free(cb);
    // 测试:再次释放，会报错，证明释放的正确
    // rs_CardBuffer_free(cb);
    rs_PokerCards_free(a);
    // 测试:再次释放，会报错，证明释放的正确
    // rs_PokerCards_free(a);
}

void test_texas() {
    rs_TexasCards *a = rs_TexasCards_new();
    printf("new texas cards address = %p\n", a);

    unsigned short hand[] = {1, 2, 3, 4, 5, 14, 15};
    char r = rs_TexasCards_assign(a, hand, 7);
    printf("assign pokercards ret = %d\n", r);

    rs_TexasCardBuffer tcb = rs_TexasCards_get_best(a);
    printf("len = %ld\n", tcb.cardbuf.len);
    for(int i=0; i<tcb.cardbuf.len; i++) {
        printf("  card %d = (%d, %d)\n", hand[i], tcb.cardbuf.data[i].suit, tcb.cardbuf.data[i].number);
    }
    printf("score = %llx\n", tcb.score);

    rs_TexasCardBuffer_free(tcb);
    rs_TexasCards_free(a);
}

void test_gin_rummy() {
    rs_GinRummyCards *gc = rs_GinRummyCards_new();
    printf("new gin_rummy cards address = %p\n", gc);

    unsigned short input[10] = {1,40, 2,3,4,5,31,32,33,41};

    // 有效的ret数据格式：
    // deadwood分数 
    // deadwood长度 deadwood1 deadwood2 ... 
    // meld1长度 meld1_1 meld1_2 ...
    // meld2长度 meld2_1 meld2_2...
    // ...
    // 长度32足够了
    unsigned char ret[32];

    // 不清零也可以
    // memset(ret, 0, 32);
    
    // 有效的返回r为ret的长度
    // 第四个参数为0表示自动排序，寻找最佳
    // 如果为1，则表示不动顺序寻找最佳
    char r = rs_GinRummyCards_assign(gc, input, 10, 0, ret);
    if (r > 0) {
        for(int i=0; i<r; i++) 
            printf("%d ", ret[i]);
        printf("\n");
        int idx = 1, ccnt = 0, mcnt = 0;
        while(ccnt < 10) {
            int mlen = ret[idx++];
            ccnt += mlen;
            if (mcnt == 0) 
                printf("deadwood(value=%d): ", ret[0]);
            else 
                printf("meld: ");
            for(int i=0; i<mlen; i++) {
                printf("%d ", ret[idx++]);
            }
            printf("\n");
            mcnt++;
        }
    }
    r = rs_GinRummyCards_sort(gc, ret);
    printf("sort...ret=%d\n", r);
    if (r > 0) {
        for(int i=0; i<r; i++) 
            printf("%d ", ret[i]);
        printf("\n");
    }
    rs_GinRummyCards_free(gc);
}

int main()
{
    test_poker_card();
    test_texas();
    test_gin_rummy();
    printf("\n");
    return 0;
}

