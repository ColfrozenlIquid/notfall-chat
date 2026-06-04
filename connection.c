#include "connection.h"
#include <stdlib.h>
#include <time.h>
#include "packet.h"

bool fsm_dispatch(Connection* conn, Event event) {
    for (size_t i = 0; i < sizeof(transitions); i++) {
        const Transition* t = &transitions[i];
        if (t->current == conn->state && t->event == event) {
            if (t->action) t->action(conn);
            conn->state = t->next;
            return true;
        }
    }
    return false;
}

void send_syn(Connection* conn) {
    conn->snd_seq = rand();
    send_segment(conn, FLAG_SYN, conn->snd_seq, 0, NULL, 0);
    conn->snd_seq++;
}

void send_ack(Connection* conn) {
    send_segment(conn, FLAG_ACK, conn->snd_seq, conn->rcv_seq, NULL, 0);
}

void send_syn_ack(Connection* conn) {
    conn->snd_seq = rand();
    send_segment(conn, FLAG_SYN | FLAG_ACK, conn->snd_seq, conn->rcv_seq, NULL, 0);
    conn->snd_seq++;
}

void send_fin(Connection* conn) {

}

void send_data(Connection* conn) {

}

void reset_listen(Connection *conn) {}

void send_segment(Connection *conn, uint16_t flag, uint32_t snd_seq, uint32_t rcv_seq, uint8_t* snd_buf, size_t snd_len) {

}
